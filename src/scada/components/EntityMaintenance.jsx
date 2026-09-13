import { useEffect, useState } from 'react';
import Modal from 'react-modal';
import Swal from 'sweetalert2';
import 'sweetalert2/dist/sweetalert2.min.css';

import { calendarApi } from '../../api';
import { getEnvVariables } from '../../helpers';

const customStyles = {
    content: {
        top: '50%',
        left: '50%',
        right: 'auto',
        bottom: 'auto',
        marginRight: '-50%',
        transform: 'translate(-50%, -50%)',
    },
};

if ( getEnvVariables().VITE_MODE !== 'test' ) {
    Modal.setAppElement('#root');
}

const EntityField = ({ field, value, formSubmitted, onChange }) => {

    const { label, type = 'text', required, options = [], placeholder, helpText, rows } = field;

    const isEmpty = value === undefined || value === null || value === '';
    const invalidClass = ( formSubmitted && required && isEmpty ) ? 'is-invalid' : '';

    const handleChange = ( event ) => {
        onChange( type === 'checkbox' ? event.target.checked : event.target.value );
    }

    if ( type === 'checkbox' ) {
        return (
            <div className="form-check mb-3">
                <input
                    type="checkbox"
                    className="form-check-input"
                    id={ `field-${ field.name }` }
                    checked={ !!value }
                    onChange={ handleChange }
                />
                <label className="form-check-label" htmlFor={ `field-${ field.name }` }>{ label }</label>
            </div>
        )
    }

    return (
        <div className="form-group mb-2">
            <label>{ label }{ required ? ' *' : '' }</label>

            { type === 'select' && (
                <select
                    className={ `form-control ${ invalidClass }` }
                    value={ value ?? '' }
                    onChange={ handleChange }
                >
                    <option value="">-- Seleccione --</option>
                    { options.map( option => (
                        <option key={ option.value } value={ option.value }>{ option.label }</option>
                    ))}
                </select>
            )}

            { type === 'textarea' && (
                <textarea
                    className={ `form-control ${ invalidClass }` }
                    rows={ rows || 4 }
                    placeholder={ placeholder }
                    value={ value ?? '' }
                    onChange={ handleChange }
                />
            )}

            { type !== 'select' && type !== 'textarea' && (
                <input
                    type={ type }
                    className={ `form-control ${ invalidClass }` }
                    placeholder={ placeholder }
                    autoComplete="off"
                    value={ value ?? '' }
                    onChange={ handleChange }
                />
            )}

            { helpText && <small className="form-text text-muted">{ helpText }</small> }
        </div>
    )
}

/**
 * Pantalla de mantenimiento (listar / crear / editar / borrar) genérica para
 * una entidad SCADA. Cada página concreta (sites, assets, ...) sólo aporta la
 * configuración de columnas/campos y el mapeo formulario <-> payload de la API;
 * esta pieza reutiliza siempre la misma tabla, modal y llamadas CRUD.
 */
export const EntityMaintenance = ({
    title,
    endpoint,
    listKey,
    columns,
    fields,
    getEmptyForm,
    toFormValues,
    toPayload,
    onItemsChange,
    rowKey = 'id',
}) => {

    const [ items, setItems ] = useState([]);
    const [ isLoading, setIsLoading ] = useState(true);
    const [ isModalOpen, setIsModalOpen ] = useState(false);
    const [ editingId, setEditingId ] = useState(null);
    const [ formValues, setFormValues ] = useState({});
    const [ formSubmitted, setFormSubmitted ] = useState(false);
    const [ isSaving, setIsSaving ] = useState(false);

    const loadItems = async() => {
        setIsLoading(true);
        try {
            const { data } = await calendarApi.get( endpoint );
            const loaded = data[listKey] || [];
            setItems( loaded );
            onItemsChange?.( loaded );
        } catch (error) {
            Swal.fire('Error al cargar', error.response?.data?.msg || error.message, 'error');
        } finally {
            setIsLoading(false);
        }
    }

    useEffect(() => {
        loadItems();
    }, [ endpoint ]);

    const openCreateModal = () => {
        setEditingId(null);
        setFormValues( getEmptyForm() );
        setFormSubmitted(false);
        setIsModalOpen(true);
    }

    const openEditModal = ( item ) => {
        setEditingId( item[rowKey] );
        setFormValues( toFormValues( item ) );
        setFormSubmitted(false);
        setIsModalOpen(true);
    }

    const closeModal = () => {
        if ( isSaving ) return;
        setIsModalOpen(false);
    }

    const onFieldChange = ( name, value ) => {
        setFormValues( current => ({ ...current, [name]: value }) );
    }

    const onSubmit = async( event ) => {
        event.preventDefault();
        setFormSubmitted(true);

        let payload;
        try {
            payload = toPayload( formValues );
        } catch (error) {
            Swal.fire('Datos inválidos', error.message, 'error');
            return;
        }

        setIsSaving(true);
        try {
            if ( editingId ) {
                await calendarApi.put(`${ endpoint }/${ editingId }`, payload );
            } else {
                await calendarApi.post( endpoint, payload );
            }
            await loadItems();
            setIsModalOpen(false);
            setFormSubmitted(false);
        } catch (error) {
            Swal.fire('Error al guardar', error.response?.data?.msg || error.message, 'error');
        } finally {
            setIsSaving(false);
        }
    }

    const onDelete = async( item ) => {
        const { isConfirmed } = await Swal.fire({
            title: '¿Eliminar registro?',
            text: 'Esta acción no se puede deshacer y puede borrar en cascada registros relacionados.',
            icon: 'warning',
            showCancelButton: true,
            confirmButtonText: 'Eliminar',
            cancelButtonText: 'Cancelar',
        });

        if ( !isConfirmed ) return;

        try {
            await calendarApi.delete(`${ endpoint }/${ item[rowKey] }`);
            await loadItems();
        } catch (error) {
            Swal.fire('Error al eliminar', error.response?.data?.msg || error.message, 'error');
        }
    }

    return (
        <div className="scada-maintenance">
            <div className="d-flex justify-content-between align-items-center mb-3">
                <h3 className="m-0">{ title }</h3>
                <button className="btn btn-primary" onClick={ openCreateModal }>
                    <i className="fas fa-plus"></i>
                    <span> Nuevo</span>
                </button>
            </div>

            { isLoading
                ? <p>Cargando…</p>
                : (
                    <div className="table-responsive">
                        <table className="table table-striped table-hover align-middle">
                            <thead>
                                <tr>
                                    { columns.map( column => <th key={ column.key }>{ column.label }</th> ) }
                                    <th>Acciones</th>
                                </tr>
                            </thead>
                            <tbody>
                                { items.length === 0 && (
                                    <tr>
                                        <td colSpan={ columns.length + 1 } className="text-center text-muted">
                                            Sin registros
                                        </td>
                                    </tr>
                                )}
                                { items.map( item => (
                                    <tr key={ item[rowKey] }>
                                        { columns.map( column => (
                                            <td key={ column.key }>
                                                { column.render ? column.render( item ) : String( item[column.key] ?? '—' ) }
                                            </td>
                                        ))}
                                        <td>
                                            <button
                                                className="btn btn-sm btn-outline-primary me-2"
                                                onClick={ () => openEditModal( item ) }
                                            >
                                                <i className="fas fa-pen"></i>
                                            </button>
                                            <button
                                                className="btn btn-sm btn-outline-danger"
                                                onClick={ () => onDelete( item ) }
                                            >
                                                <i className="fas fa-trash-alt"></i>
                                            </button>
                                        </td>
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    </div>
                )
            }

            <Modal
                isOpen={ isModalOpen }
                onRequestClose={ closeModal }
                style={ customStyles }
                className="modal modal-scada"
                overlayClassName="modal-fondo"
                closeTimeoutMS={ 200 }
            >
                <h1>{ editingId ? `Editar ${ title }` : `Nuevo en ${ title }` }</h1>
                <hr />
                <form className="container" onSubmit={ onSubmit }>
                    { fields.map( field => (
                        <EntityField
                            key={ field.name }
                            field={ field }
                            value={ formValues[field.name] }
                            formSubmitted={ formSubmitted }
                            onChange={ ( value ) => onFieldChange( field.name, value ) }
                        />
                    ))}

                    <button
                        type="submit"
                        className="btn btn-outline-primary btn-block"
                        disabled={ isSaving }
                    >
                        <i className="far fa-save"></i>
                        <span> { isSaving ? 'Guardando…' : 'Guardar' }</span>
                    </button>
                </form>
            </Modal>
        </div>
    )
}
