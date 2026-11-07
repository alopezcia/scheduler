import { fireEvent, render, screen, waitFor } from '@testing-library/react';

import { UsersPage } from '../../../src/scada/pages/UsersPage';
import { calendarApi } from '../../../src/api';
import { useAuthStore } from '../../../src/hooks';
import Swal from 'sweetalert2';

jest.mock('../../../src/api', () => ({ calendarApi: { get: jest.fn(), put: jest.fn(), post: jest.fn() } }));
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
        calendarApi.post.mockResolvedValue({ data: { ok: true } });
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

    test('crea un usuario con los datos del formulario y recarga la lista', async() => {
        render(<UsersPage />);
        await screen.findByText('luis@x.com');

        fireEvent.click( screen.getByRole('button', { name: /Nuevo/ }) );
        fireEvent.change( screen.getByPlaceholderText('Nombre'), { target: { name: 'name', value: 'Pía' } } );
        fireEvent.change( screen.getByPlaceholderText('Correo'), { target: { name: 'email', value: 'pia@x.com' } } );
        fireEvent.change( screen.getByPlaceholderText(/Contraseña/), { target: { name: 'password', value: 'secreto1' } } );
        fireEvent.change( screen.getByLabelText('Rol del nuevo usuario'), { target: { name: 'role', value: 'planificador' } } );
        fireEvent.submit( screen.getByRole('form', { name: 'Nuevo usuario' }) );

        await waitFor(() => expect( calendarApi.post ).toHaveBeenCalledWith('/users', {
            name: 'Pía', email: 'pia@x.com', password: 'secreto1', role: 'planificador',
        }) );
        await waitFor(() => expect( calendarApi.get ).toHaveBeenCalledTimes(2) );
        expect( screen.queryByRole('form', { name: 'Nuevo usuario' }) ).toBeNull();
    });

    test('si el backend rechaza el alta, avisa y conserva el formulario', async() => {
        calendarApi.post.mockRejectedValue({ response: { data: { msg: 'El usuario ya existe' } } });
        render(<UsersPage />);
        await screen.findByText('luis@x.com');

        fireEvent.click( screen.getByRole('button', { name: /Nuevo/ }) );
        fireEvent.change( screen.getByPlaceholderText('Nombre'), { target: { name: 'name', value: 'Luis' } } );
        fireEvent.change( screen.getByPlaceholderText('Correo'), { target: { name: 'email', value: 'luis@x.com' } } );
        fireEvent.change( screen.getByPlaceholderText(/Contraseña/), { target: { name: 'password', value: 'secreto1' } } );
        fireEvent.submit( screen.getByRole('form', { name: 'Nuevo usuario' }) );

        await waitFor(() => expect( Swal.fire ).toHaveBeenCalledWith('No se pudo crear el usuario', 'El usuario ya existe', 'error') );
        expect( screen.getByRole('form', { name: 'Nuevo usuario' }) ).toBeTruthy();
    });
});
