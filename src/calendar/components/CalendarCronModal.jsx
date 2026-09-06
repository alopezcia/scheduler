import { useState } from 'react';
import { addMinutes } from 'date-fns';

import Swal from 'sweetalert2';
import 'sweetalert2/dist/sweetalert2.min.css';

import Modal from 'react-modal';

import DatePicker, { registerLocale } from 'react-datepicker';
import 'react-datepicker/dist/react-datepicker.css';

import es from 'date-fns/locale/es';
import { useCalendarStore, useUiStore } from '../../hooks';
import { generateCronDates, getEnvVariables, MAX_CRON_ENTRIES } from '../../helpers';

registerLocale( 'es', es );

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

if ( getEnvVariables().VITE_MODE !== 'test'  ) {
    Modal.setAppElement('#root');
}

const initialFormState = {
    title: '',
    notes: '',
    cronExpression: '',
    start: new Date(),
    duration: 60,
    entries: 5,
};

export const CalendarCronModal = () => {

    const { isCronModalOpen, closeCronModal } = useUiStore();
    const { startSavingManyEvents } = useCalendarStore();

    const [ formSubmitted, setFormSubmitted ] = useState(false);
    const [ isSaving, setIsSaving ] = useState(false);
    const [ formValues, setFormValues ] = useState({ ...initialFormState });

    const onInputChanged = ({ target }) => {
        setFormValues({
            ...formValues,
            [target.name]: target.value
        })
    }

    const onDateChanged = ( event ) => {
        setFormValues({
            ...formValues,
            start: event
        })
    }

    const onCloseModal = () => {
        if ( isSaving ) return;
        closeCronModal();
        setFormSubmitted(false);
        setFormValues({ ...initialFormState });
    }

    const onSubmit = async( event ) => {
        event.preventDefault();
        setFormSubmitted(true);

        if ( formValues.title.length <= 0 ) return;

        let dates;

        try {
            dates = generateCronDates({
                expression: formValues.cronExpression,
                startDate: formValues.start,
                count: formValues.entries,
            });
        } catch (error) {
            Swal.fire('Expresión crontab inválida', error.message, 'error');
            return;
        }

        const duration = Number( formValues.duration );

        if ( isNaN( duration ) || duration <= 0 ) {
            Swal.fire('Duración incorrecta', 'La duración debe ser mayor a 0 minutos', 'error');
            return;
        }

        const newEvents = dates.map( date => ({
            title: formValues.title,
            notes: formValues.notes,
            start: date,
            end: addMinutes( date, duration ),
        }));

        setIsSaving(true);
        const { successCount, errorCount } = await startSavingManyEvents( newEvents );
        setIsSaving(false);

        if ( errorCount > 0 ) {
            Swal.fire(
                'Creación parcial',
                `Se crearon ${ successCount } de ${ newEvents.length } eventos. ${ errorCount } fallaron.`,
                'warning'
            );
        } else {
            Swal.fire('Eventos creados', `Se crearon ${ successCount } eventos correctamente`, 'success');
        }

        closeCronModal();
        setFormSubmitted(false);
        setFormValues({ ...initialFormState });
    }

  return (
    <Modal
        isOpen={ isCronModalOpen }
        onRequestClose={ onCloseModal }
        style={ customStyles }
        className="modal"
        overlayClassName="modal-fondo"
        closeTimeoutMS={ 200 }
    >
        <h1> Crear eventos desde crontab </h1>
        <hr />
        <form className="container" onSubmit={ onSubmit }>

            <div className="form-group mb-2">
                <label>Expresión crontab</label>
                <input
                    type="text"
                    className={ `form-control ${ ( formSubmitted && formValues.cronExpression.length <= 0 ) ? 'is-invalid' : '' }`}
                    placeholder="Ej. 0 9 * * 1-5"
                    name="cronExpression"
                    autoComplete="off"
                    value={ formValues.cronExpression }
                    onChange={ onInputChanged }
                />
                <small className="form-text text-muted">Formato estándar: minuto hora día-mes mes día-semana</small>
            </div>

            <div className="form-group mb-2">
                <label>Fecha de inicio</label>
                <DatePicker
                    selected={ formValues.start }
                    onChange={ onDateChanged }
                    className="form-control"
                    dateFormat="Pp"
                    showTimeSelect
                    locale="es"
                    timeCaption="Hora"
                />
                <small className="form-text text-muted">Se generarán las siguientes ocurrencias a partir de esta fecha</small>
            </div>

            <div className="form-group mb-2">
                <label>Número de entradas a crear</label>
                <input
                    type="number"
                    className="form-control"
                    name="entries"
                    min="1"
                    max={ MAX_CRON_ENTRIES }
                    value={ formValues.entries }
                    onChange={ onInputChanged }
                />
            </div>

            <div className="form-group mb-2">
                <label>Duración de cada evento (minutos)</label>
                <input
                    type="number"
                    className="form-control"
                    name="duration"
                    min="1"
                    value={ formValues.duration }
                    onChange={ onInputChanged }
                />
            </div>

            <hr />
            <div className="form-group mb-2">
                <label>Título y notas</label>
                <input
                    type="text"
                    className={ `form-control ${ ( formSubmitted && formValues.title.length <= 0 ) ? 'is-invalid' : '' }`}
                    placeholder="Título del evento"
                    name="title"
                    autoComplete="off"
                    value={ formValues.title }
                    onChange={ onInputChanged }
                />
                <small className="form-text text-muted">Se usará para todos los eventos generados</small>
            </div>

            <div className="form-group mb-2">
                <textarea
                    type="text"
                    className="form-control"
                    placeholder="Notas"
                    rows="3"
                    name="notes"
                    value={ formValues.notes }
                    onChange={ onInputChanged }
                ></textarea>
            </div>

            <button
                type="submit"
                className="btn btn-outline-primary btn-block"
                disabled={ isSaving }
            >
                <i className="fas fa-business-time"></i>
                <span> { isSaving ? 'Creando…' : 'Generar eventos' } </span>
            </button>

        </form>
    </Modal>
  )
}
