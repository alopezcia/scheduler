import { buildSocketUrl } from '../../src/api/schedulerSocket';

describe('buildSocketUrl', () => {

    test('con una API absoluta cambia http por ws', () => {
        expect( buildSocketUrl('http://localhost:4000/api') ).toBe('ws://localhost:4000/api/scheduler/ws');
        expect( buildSocketUrl('https://example.com/api/') ).toBe('wss://example.com/api/scheduler/ws');
    });

    test('con una API relativa usa el origen de la página', () => {
        expect( buildSocketUrl('/api', { protocol: 'http:', host: 'localhost:8080' }) ).toBe('ws://localhost:8080/api/scheduler/ws');
        expect( buildSocketUrl('/api', { protocol: 'https:', host: 'app.example.com' }) ).toBe('wss://app.example.com/api/scheduler/ws');
    });
});

describe('reconexión del WebSocket', () => {

    const { connectSchedulerSocket, disconnectSchedulerSocket, CLOSE_UNAUTHORIZED } = require('../../src/api/schedulerSocket');
    let sockets;

    beforeEach(() => {
        jest.useFakeTimers();
        sockets = [];
        global.WebSocket = class {
            constructor() { sockets.push( this ); }
            send() {}
            close() {}
        };
        global.WebSocket.OPEN = 1;
    });

    afterEach(() => {
        disconnectSchedulerSocket();
        jest.useRealTimers();
    });

    test('tras una caída reintenta con espera', () => {
        const dispatch = jest.fn();
        connectSchedulerSocket( dispatch );
        sockets[0].onclose({ code: 1006 });

        expect( dispatch ).toHaveBeenLastCalledWith({ type: 'scheduler/onSocketState', payload: 'closed' });
        jest.advanceTimersByTime( 1000 );
        expect( sockets ).toHaveLength( 2 );
    });

    test('con el token rechazado (4401) no reintenta y lo indica', () => {
        const dispatch = jest.fn();
        connectSchedulerSocket( dispatch );
        sockets[0].onclose({ code: CLOSE_UNAUTHORIZED });

        expect( dispatch ).toHaveBeenLastCalledWith({ type: 'scheduler/onSocketState', payload: 'unauthorized' });
        jest.advanceTimersByTime( 60000 );
        expect( sockets ).toHaveLength( 1 );
    });
});
