import { getEnvVariables } from '../helpers';
import {
    onSchedulerExecution, onSchedulerLink, onSchedulerStatus, onSocketState,
} from '../store/scheduler/schedulerSlice';

// Canal con el backend (nunca con la app scheduler directamente): el backend lo reenvía
// por su socket Unix. Ver backend/src/scheduler_ws.rs para el protocolo.

const RECONNECT_DELAYS_MS = [ 1000, 2000, 5000, 10000 ];
const REQUEST_TIMEOUT_MS = 8000;
// Código con el que el backend cierra cuando el token no vale (ver scheduler_ws.rs)
export const CLOSE_UNAUTHORIZED = 4401;

// El WebSocket cuelga de la misma base que la API REST: '/api' (mismo origen, tras nginx)
// o una URL absoluta http(s):// en desarrollo.
export const buildSocketUrl = ( apiUrl, location = window.location ) => {
    const base = /^https?:\/\//.test( apiUrl )
        ? apiUrl.replace( /^http/, 'ws' )
        : `${ location.protocol === 'https:' ? 'wss' : 'ws' }://${ location.host }${ apiUrl }`;
    return `${ base.replace( /\/$/, '' ) }/scheduler/ws`;
}

let socket = null;
let dispatch = null;
let retries = 0;
let retryTimer = null;
let wanted = false;
let nextId = 1;
const pending = new Map();

const rejectPending = ( message ) => {
    pending.forEach( ({ reject, timer }) => { clearTimeout( timer ); reject( new Error( message ) ); });
    pending.clear();
}

const handleMessage = ( message ) => {
    switch ( message.type ) {
        case 'auth':
            retries = 0;
            dispatch( onSocketState('open') );
            break;
        case 'link':
            dispatch( onSchedulerLink( message.connected ) );
            break;
        case 'status':
            dispatch( onSchedulerStatus( message ) );
            break;
        case 'execution':
            dispatch( onSchedulerExecution( message ) );
            break;
        case 'reply': {
            const request = pending.get( message.id );
            if ( !request ) break;
            pending.delete( message.id );
            clearTimeout( request.timer );
            if ( message.ok ) request.resolve();
            else request.reject( new Error( message.error || 'El scheduler rechazó la orden' ) );
            break;
        }
        default:
            break;
    }
}

const open = () => {
    const { VITE_API_URL } = getEnvVariables();
    dispatch( onSocketState('connecting') );
    const ws = new WebSocket( buildSocketUrl( VITE_API_URL ) );
    socket = ws;

    // El token va en el primer mensaje y no en la URL, para que no quede en logs.
    ws.onopen = () => ws.send( JSON.stringify({ type: 'auth', token: localStorage.getItem('token') }) );
    ws.onmessage = ( event ) => {
        try {
            handleMessage( JSON.parse( event.data ) );
        } catch {
            // Mensaje que no es JSON: se ignora.
        }
    };
    ws.onclose = ( event ) => {
        if ( socket !== ws ) return;
        socket = null;
        rejectPending('Se perdió la conexión con el servidor');
        // Con un token inválido reintentar es inútil: hay que volver a iniciar sesión
        const unauthorized = event?.code === CLOSE_UNAUTHORIZED;
        dispatch( onSocketState( unauthorized ? 'unauthorized' : 'closed' ) );
        if ( wanted && !unauthorized ) {
            const delay = RECONNECT_DELAYS_MS[ Math.min( retries, RECONNECT_DELAYS_MS.length - 1 ) ];
            retries += 1;
            retryTimer = setTimeout( open, delay );
        }
    };
}

export const connectSchedulerSocket = ( storeDispatch ) => {
    dispatch = storeDispatch;
    wanted = true;
    if ( !socket ) open();
}

export const disconnectSchedulerSocket = () => {
    wanted = false;
    clearTimeout( retryTimer );
    if ( socket ) {
        const ws = socket;
        socket = null;
        ws.close();
        rejectPending('Conexión cerrada');
        dispatch( onSocketState('closed') );
    }
}

// Envía una orden ({ cmd, execution_id? }) y resuelve cuando el scheduler la confirma.
export const sendSchedulerCommand = ( command ) => new Promise(( resolve, reject ) => {
    if ( !socket || socket.readyState !== WebSocket.OPEN ) {
        return reject( new Error('No hay conexión con el servidor') );
    }
    const id = String( nextId++ );
    const timer = setTimeout(() => {
        pending.delete( id );
        reject( new Error('El servidor no respondió a tiempo') );
    }, REQUEST_TIMEOUT_MS );
    pending.set( id, { resolve, reject, timer } );
    socket.send( JSON.stringify({ id, ...command }) );
});
