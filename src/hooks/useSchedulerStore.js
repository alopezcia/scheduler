import { useCallback, useEffect } from 'react';
import { useDispatch, useSelector } from 'react-redux';
import Swal from 'sweetalert2';

import { useAuthStore } from './useAuthStore';
import { connectSchedulerSocket, disconnectSchedulerSocket, sendSchedulerCommand } from '../api/schedulerSocket';


// Mientras algún componente usa este hook con `autoConnect`, hay un WebSocket abierto.
export const useSchedulerStore = ({ autoConnect = false } = {}) => {

    const dispatch = useDispatch();
    const { socket, linked, paused, running, log } = useSelector( state => state.scheduler );
    const { isAdmin } = useAuthStore();

    useEffect(() => {
        if ( !autoConnect ) return;
        connectSchedulerSocket( dispatch );
        return () => disconnectSchedulerSocket();
    }, [ autoConnect, dispatch ]);

    const run = useCallback( async( command ) => {
        try {
            await sendSchedulerCommand( command );
        } catch ( error ) {
            Swal.fire('Scheduler', error.message, 'error');
        }
    }, []);

    return {
        //* Propiedades
        // Se pueden mandar órdenes si hay WebSocket, el backend ve al scheduler y el usuario es administrador
        // (el backend lo vuelve a comprobar en cada orden)
        canControl: isAdmin && socket === 'open' && linked === true,
        isAdmin,
        linked,
        log,
        paused,
        running,
        socket,

        //* Métodos
        cancelExecution: async( executionId ) => {
            const { isConfirmed } = await Swal.fire({
                title: '¿Cancelar la ejecución?',
                text: 'Si el comando ya salió hacia el SCADA, este puede haberlo recibido y aplicado igualmente.',
                icon: 'warning',
                showCancelButton: true,
                confirmButtonText: 'Cancelar ejecución',
                cancelButtonText: 'Volver',
            });
            if ( isConfirmed ) await run({ cmd: 'cancel', execution_id: executionId });
        },
        pause: () => run({ cmd: 'pause' }),
        resume: () => run({ cmd: 'resume' }),
    }
}
