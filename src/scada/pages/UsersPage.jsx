import { useCallback, useEffect, useState } from 'react';
import Swal from 'sweetalert2';

import { calendarApi } from '../../api';
import { useAuthStore } from '../../hooks';

export const ROLES = [
    { value: 'consultor', label: 'Consultor', help: 'Ve todo lo registrado' },
    { value: 'planificador', label: 'Planificador', help: 'Además crea y modifica eventos y configuración' },
    { value: 'administrador', label: 'Administrador', help: 'Además controla el scheduler y gestiona usuarios' },
];

const EMPTY_USER = { name: '', email: '', password: '', role: 'consultor' };

export const UsersPage = () => {

    const { user, checkAuthToken } = useAuthStore();
    const [ users, setUsers ] = useState([]);
    const [ isLoading, setIsLoading ] = useState(true);
    const [ savingId, setSavingId ] = useState(null);
    const [ isCreating, setIsCreating ] = useState(false);
    const [ isSavingNew, setIsSavingNew ] = useState(false);
    const [ newUser, setNewUser ] = useState( EMPTY_USER );

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

    const onNewUserChange = ({ target }) => setNewUser( current => ({ ...current, [ target.name ]: target.value }) );

    const onCreate = async( event ) => {
        event.preventDefault();
        setIsSavingNew( true );
        try {
            await calendarApi.post('/users', newUser );
            setNewUser( EMPTY_USER );
            setIsCreating( false );
            await loadUsers();
        } catch ( error ) {
            Swal.fire('No se pudo crear el usuario', error.response?.data?.msg || error.message, 'error');
        } finally {
            setIsSavingNew( false );
        }
    }

    return (
        <div className="scada-maintenance">
            <div className="d-flex justify-content-between align-items-center">
                <h3>Usuarios</h3>
                { !isCreating && (
                    <button type="button" className="btn btn-primary" onClick={ () => setIsCreating( true ) }>
                        <i className="fas fa-plus"></i>
                        <span> Nuevo</span>
                    </button>
                )}
            </div>

            { isCreating && (
                <form className="card card-body mb-3" onSubmit={ onCreate } aria-label="Nuevo usuario">
                    <div className="row g-2">
                        <div className="col-md-3">
                            <input name="name" className="form-control" placeholder="Nombre" required
                                value={ newUser.name } onChange={ onNewUserChange } />
                        </div>
                        <div className="col-md-3">
                            <input name="email" type="email" className="form-control" placeholder="Correo" required
                                value={ newUser.email } onChange={ onNewUserChange } />
                        </div>
                        <div className="col-md-2">
                            <input name="password" type="password" className="form-control" placeholder="Contraseña (mín. 6)"
                                required minLength={ 6 } autoComplete="new-password"
                                value={ newUser.password } onChange={ onNewUserChange } />
                        </div>
                        <div className="col-md-2">
                            <select name="role" aria-label="Rol del nuevo usuario" className="form-select"
                                value={ newUser.role } onChange={ onNewUserChange }>
                                { ROLES.map( role => <option key={ role.value } value={ role.value }>{ role.label }</option> ) }
                            </select>
                        </div>
                        <div className="col-md-2 d-flex gap-2">
                            <button type="submit" className="btn btn-primary" disabled={ isSavingNew }>Crear</button>
                            <button type="button" className="btn btn-outline-secondary" disabled={ isSavingNew }
                                onClick={ () => { setIsCreating( false ); setNewUser( EMPTY_USER ); } }>
                                Cancelar
                            </button>
                        </div>
                    </div>
                </form>
            )}

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
