import { configureStore } from '@reduxjs/toolkit';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { Provider } from 'react-redux';
import Swal from 'sweetalert2';

import { calendarApi } from '../../../src/api';
import { ScadaPage } from '../../../src/scada/pages/ScadaPage';
import { authSlice } from '../../../src/store';

// Integración de la pestaña Usuarios: ScadaPage + UsersPage + useAuthStore + store real,
// con un backend simulado que aplica las mismas reglas que el verdadero (el último
// administrador no se puede degradar y el rol de cada usuario se lee de "la base de datos").

jest.mock('../../../src/api', () => ({ calendarApi: { get: jest.fn(), put: jest.fn() } }));
jest.mock('sweetalert2', () => ({ fire: jest.fn() }));
jest.mock('../../../src/calendar', () => ({ Navbar: () => null }));
jest.mock('../../../src/scada/pages/SitesMaintenancePage', () => ({ SitesMaintenancePage: () => <p>Sites page</p> }));
jest.mock('../../../src/scada/pages/AssetsMaintenancePage', () => ({ AssetsMaintenancePage: () => <p>Assets page</p> }));
jest.mock('../../../src/scada/pages/ConnectionsMaintenancePage', () => ({ ConnectionsMaintenancePage: () => <p>Connections page</p> }));
jest.mock('../../../src/scada/pages/TagsMaintenancePage', () => ({ TagsMaintenancePage: () => <p>Tags page</p> }));
jest.mock('../../../src/scada/pages/SchedulesMaintenancePage', () => ({ SchedulesMaintenancePage: () => <p>Schedules page</p> }));
jest.mock('../../../src/scada/pages/SchedulerPage', () => ({ SchedulerPage: () => <p>Scheduler page</p> }));

let db;

const fakeBackend = ( currentUid ) => {
    calendarApi.get.mockImplementation( async( url ) => {
        if ( url === '/users' ) return { data: { ok: true, users: db.map( u => ({ ...u }) ) } };
        if ( url === 'auth/renew' ) {
            const me = db.find( u => u.id === currentUid );
            return { data: { ok: true, uid: me.id, name: me.name, role: me.role, token: 'nuevo' } };
        }
        throw new Error(`GET inesperado: ${ url }`);
    });
    calendarApi.put.mockImplementation( async( url, { role } ) => {
        const target = db.find( u => u.id === url.split('/')[2] );
        const admins = db.filter( u => u.role === 'administrador' ).length;
        if ( target.role === 'administrador' && role !== 'administrador' && admins <= 1 ) {
            throw { response: { data: { msg: 'Debe quedar al menos un administrador' } } };
        }
        target.role = role;
        return { data: { ok: true } };
    });
}

const renderAs = ( uid ) => {
    const me = db.find( u => u.id === uid );
    const store = configureStore({
        reducer: { auth: authSlice.reducer },
        preloadedState: { auth: { status: 'authenticated', user: { uid: me.id, name: me.name, role: me.role }, errorMessage: undefined } },
    });
    fakeBackend( uid );
    return render( <Provider store={ store }><ScadaPage /></Provider> );
}

const openUsersTab = async() => {
    fireEvent.click( screen.getByRole('button', { name: 'Usuarios' }) );
    await screen.findByText('luis@x.com');
}

describe('Pestaña Usuarios (integración)', () => {

    beforeEach(() => {
        jest.clearAllMocks();
        localStorage.setItem('token', 'viejo');
        db = [
            { id: 'u1', name: 'Ana', email: 'ana@x.com', role: 'administrador' },
            { id: 'u2', name: 'Luis', email: 'luis@x.com', role: 'consultor' },
        ];
    });

    test('un administrador abre la pestaña y ve a todos los usuarios con su rol', async() => {
        renderAs('u1');
        await openUsersTab();

        expect( screen.getByLabelText('Rol de Ana').value ).toBe('administrador');
        expect( screen.getByLabelText('Rol de Luis').value ).toBe('consultor');
        expect( screen.getByText('Tú') ).toBeTruthy();
    });

    test('un consultor o un planificador no ven la pestaña ni llaman a /users', () => {
        [ 'u2', 'u3' ].forEach( uid => {
            db = [ ...db.slice(0, 2), { id: 'u3', name: 'Pía', email: 'pia@x.com', role: 'planificador' } ];
            const { unmount } = renderAs( uid );
            expect( screen.queryByRole('button', { name: 'Usuarios' }) ).toBeNull();
            expect( screen.getByRole('button', { name: 'Scheduler' }) ).toBeTruthy();
            unmount();
        });
        expect( calendarApi.get ).not.toHaveBeenCalledWith('/users');
    });

    test('cambiar el rol de otro usuario lo guarda y la lista lo refleja', async() => {
        renderAs('u1');
        await openUsersTab();

        fireEvent.change( screen.getByLabelText('Rol de Luis'), { target: { value: 'planificador' } } );

        await waitFor(() => expect( screen.getByLabelText('Rol de Luis').value ).toBe('planificador') );
        expect( db[1].role ).toBe('planificador');
        expect( Swal.fire ).not.toHaveBeenCalled();
    });

    test('degradar al último administrador se rechaza, se avisa y el selector no cambia', async() => {
        renderAs('u1');
        await openUsersTab();

        fireEvent.change( screen.getByLabelText('Rol de Ana'), { target: { value: 'consultor' } } );

        await waitFor(() => expect( Swal.fire ).toHaveBeenCalledWith('No se pudo cambiar el rol', 'Debe quedar al menos un administrador', 'error') );
        await waitFor(() => expect( screen.getByLabelText('Rol de Ana').value ).toBe('administrador') );
        expect( db[0].role ).toBe('administrador');
    });

    test('al degradarse a sí mismo (habiendo otro administrador) la pestaña desaparece', async() => {
        db[1].role = 'administrador';
        renderAs('u1');
        await openUsersTab();

        fireEvent.change( screen.getByLabelText('Rol de Ana'), { target: { value: 'consultor' } } );

        // Se renueva la sesión (auth/renew devuelve el rol nuevo) y la interfaz lo refleja
        await waitFor(() => expect( screen.queryByRole('button', { name: 'Usuarios' }) ).toBeNull() );
        expect( calendarApi.get ).toHaveBeenCalledWith('auth/renew');
        expect( screen.getByText('Sites page') ).toBeTruthy();
    });
});
