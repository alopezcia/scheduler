import { createSlice } from '@reduxjs/toolkit';

const MAX_LOG_ENTRIES = 50;

export const schedulerSlice = createSlice({
    name: 'scheduler',
    initialState: {
        // Estado del WebSocket con el backend: 'connecting' | 'open' | 'closed' | 'unauthorized'
        socket: 'closed',
        // ¿Tiene el backend conexión con la app scheduler? null = aún no se sabe
        linked: null,
        paused: false,
        running: [],
        log: [],
    },
    reducers: {
        onSocketState: ( state, { payload } ) => {
            state.socket = payload;
            if ( payload !== 'open' ) {
                state.linked = null;
                state.running = [];
            }
        },
        onSchedulerLink: ( state, { payload } ) => {
            state.linked = payload;
            if ( !payload ) state.running = [];
        },
        onSchedulerStatus: ( state, { payload } ) => {
            state.paused = payload.paused;
            state.running = payload.running;
        },
        onSchedulerExecution: ( state, { payload } ) => {
            state.log = [ { ...payload, at: new Date().toISOString() }, ...state.log ].slice( 0, MAX_LOG_ENTRIES );
        },
    }
});

export const { onSocketState, onSchedulerLink, onSchedulerStatus, onSchedulerExecution } = schedulerSlice.actions;
