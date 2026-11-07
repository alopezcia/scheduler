import { fireEvent, render, screen, waitFor } from '@testing-library/react';

import { UsersPage } from '../../../src/scada/pages/UsersPage';
import { calendarApi } from '../../../src/api';
import { useAuthStore } from '../../../src/hooks';

jest.mock('../../../src/api', () => ({ calendarApi: { get: jest.fn(), put: jest.fn() } }));
jest.mock('../../../src/hooks', () => ({ useAuthStore: jest.fn() }));
jest.mock('sweetalert2', () => ({ fire: jest.fn() }));

const USERS = [
    { id: 'u1', name: 'Ana', email: 'ana@x.com', role: 'administrador' },
    { id: 'u2', name: 'Luis', email: 'luis@x.com', role: 'consultor' },
];

describe('Pruebas en <UsersPage />', () => {

    const checkAuthToken = jest.fn();

    beforeEach(() => {
        jest.clearAllMocks();
        useAuthStore.mockReturnValue({ user: { uid: 'u1' }, checkAuthToken });
        calendarApi.get.mockResolvedValue({ data: { users: USERS } });
        calendarApi.put.mockResolvedValue({ data: { ok: true } });
    });

    test('lista los usuarios con su rol actual', async() => {
        render(<UsersPage />);

        expect( await screen.findByText('luis@x.com') ).toBeTruthy();
        expect( screen.getByLabelText('Rol de Luis').value ).toBe('consultor');
        expect( screen.getByLabelText('Rol de Ana').value ).toBe('administrador');
    });

    test('cambiar el rol de otro usuario llama al backend y recarga', async() => {
        render(<UsersPage />);
        const select = await screen.findByLabelText('Rol de Luis');

        fireEvent.change( select, { target: { value: 'planificador' } } );

        await waitFor(() => expect( calendarApi.put ).toHaveBeenCalledWith('/users/u2/role', { role: 'planificador' }) );
        await waitFor(() => expect( calendarApi.get ).toHaveBeenCalledTimes(2) );
        expect( checkAuthToken ).not.toHaveBeenCalled();
    });

    test('si cambia su propio rol renueva la sesión', async() => {
        render(<UsersPage />);
        const select = await screen.findByLabelText('Rol de Ana');

        fireEvent.change( select, { target: { value: 'planificador' } } );

        await waitFor(() => expect( checkAuthToken ).toHaveBeenCalledTimes(1) );
    });
});
