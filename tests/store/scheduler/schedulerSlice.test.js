import { schedulerSlice, onSocketState, onSchedulerLink, onSchedulerStatus, onSchedulerExecution } from '../../../src/store/scheduler/schedulerSlice';

describe('schedulerSlice', () => {

    const running = [{ execution_id: 'x1', schedule_name: 'S', tag_name: 'T', started_at: '2026-01-01T00:00:00Z' }];

    test('guarda el estado que envía el scheduler', () => {
        const state = schedulerSlice.reducer( undefined, onSchedulerStatus({ paused: true, running }) );
        expect( state.paused ).toBe( true );
        expect( state.running ).toEqual( running );
    });

    test('al perder el scheduler o el socket se vacían las ejecuciones en curso', () => {
        let state = schedulerSlice.reducer( undefined, onSchedulerStatus({ paused: false, running }) );
        state = schedulerSlice.reducer( state, onSchedulerLink( false ) );
        expect( state.linked ).toBe( false );
        expect( state.running ).toEqual( [] );

        state = schedulerSlice.reducer( undefined, onSchedulerStatus({ paused: false, running }) );
        state = schedulerSlice.reducer( state, onSocketState('closed') );
        expect( state.linked ).toBeNull();
        expect( state.running ).toEqual( [] );
    });

    test('el registro de actividad conserva las últimas 50 entradas, la más reciente primero', () => {
        let state = schedulerSlice.reducer( undefined, { type: 'noop' } );
        for ( let i = 0; i < 60; i++ ) {
            state = schedulerSlice.reducer( state, onSchedulerExecution({ execution_id: `e${ i }`, status: 'started' }) );
        }
        expect( state.log ).toHaveLength( 50 );
        expect( state.log[0].execution_id ).toBe('e59');
    });
});
