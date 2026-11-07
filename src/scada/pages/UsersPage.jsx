import { useCallback, useEffect, useState } from 'react';
import Swal from 'sweetalert2';

import { calendarApi } from '../../api';
import { useAuthStore } from '../../hooks';

export const ROLES = [
    { value: 'consultor', label: 'Consultor', help: 'Ve todo lo registrado' },
    { value: 'planificador', label: 'Planificador', help: 'Además crea y modifica eventos y configuración' },
    { value: 'administrador', label: 'Administrador', help: 'Además controla el scheduler y gestiona usuarios' },
];

export const UsersPage = () => {

    const { user, checkAuthToken } = useAuthStore();
    const [ users, setUsers ] = useState([]);
    const [ isLoading, setIsLoading ] = useState(true);
    const [ savingId, setSavingId ] = useState(null);

    const loadUsers = useCallback( async() => {
        try {
            const { data } = await calendarApi.get('/users');
            setUsers( data.users );
        } catch ( error ) {
            Swal.fire('Error al cargar', error.response?.data?.msg || error.message, 'error');
        } finally {
            setIsLoading( false );
        }
    }, []);

    useEffect(() => { loadUsers(); }, [ loadUsers ]);

    const onRoleChange = async( target, role ) => {
        setSavingId( target.id );
        try {
            await calendarApi.put(`/users/${ target.id }/role`, { role });
            // Si cambia su propio rol, se renueva la sesión para que la interfaz refleje el nuevo
            if ( target.id === user.uid ) await checkAuthToken();
        } catch ( error ) {
            Swal.fire('No se pudo cambiar el rol', error.response?.data?.msg || error.message, 'error');
        } finally {
            setSavingId( null );
            // Se recarga siempre: tras un error el selector vuelve al valor real
            await loadUsers();
        }
    }

    return (
        <div className="scada-maintenance">
            <h3>Usuarios</h3>
            <p className="text-muted">
                { ROLES.map( role => `${ role.label }: ${ role.help }` ).join(' · ') }
            </p>

            { isLoading
                ? <p>Cargando…</p>
                : (
                    <table className="table table-sm align-middle">
                        <thead>
                            <tr>
                                <th>Nombre</th>
                                <th>Correo</th>
                                <th>Rol</th>
                            </tr>
                        </thead>
                        <tbody>
                            { users.map( item => (
                                <tr key={ item.id }>
                                    <td>
                                        { item.name }
                                        { item.id === user.uid && <span className="badge bg-secondary ms-2">Tú</span> }
                                    </td>
                                    <td>{ item.email }</td>
                                    <td>
                                        <select
                                            aria-label={ `Rol de ${ item.name }` }
                                            className="form-select form-select-sm w-auto"
                                            value={ item.role }
                                            disabled={ savingId === item.id }
                                            onChange={ ( event ) => onRoleChange( item, event.target.value ) }
                                        >
                                            { ROLES.map( role => (
                                                <option key={ role.value } value={ role.value }>{ role.label }</option>
                                            ))}
                                        </select>
                                    </td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                )}
        </div>
    )
}
