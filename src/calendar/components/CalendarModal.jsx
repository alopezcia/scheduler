import { useEffect, useMemo, useState } from 'react';

import Modal from 'react-modal';

import DatePicker, { registerLocale } from 'react-datepicker';
import 'react-datepicker/dist/react-datepicker.css';

import es from 'date-fns/locale/es';
import { calendarApi } from '../../api';
import { useCalendarStore, useUiStore } from '../../hooks';
import { getEnvVariables } from '../../helpers';


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

export const CalendarModal = () => {

    const { isDateModalOpen, closeDateModal } = useUiStore();
    const { activeEvent, startSavingEvent } = useCalendarStore();

    const [ formSubmitted, setFormSubmitted ] = useState(false);
    const [ schedules, setSchedules ] = useState([]);

    const [formValues, setFormValues] = useState({
        schedule_id: '',
        start: new Date(),
    });

    useEffect(() => {
        calendarApi.get('/schedules')
            .then(({ data }) => setSchedules( data.schedules || [] ))
            .catch(() => setSchedules([]));
    }, []);

    const scheduleClass = useMemo(() => {
        if ( !formSubmitted ) return '';

        return ( formValues.schedule_id )
            ? ''
            : 'is-invalid';

    }, [ formValues.schedule_id, formSubmitted ])

    useEffect(() => {
      if ( activeEvent !== null ) {
          setFormValues({ ...activeEvent });
      }

    }, [ activeEvent ])



    const onScheduleChanged = ({ target }) => {
        setFormValues({
            ...formValues,
            schedule_id: target.value
        })
    }

    const onDateChanged = ( event ) => {
        setFormValues({
            ...formValues,
            start: event
        })
    }

    const onCloseModal = () => {
        closeDateModal();
    }

    const onSubmit = async( event ) => {
        event.preventDefault();
        setFormSubmitted(true);

        if ( !formValues.schedule_id ) return;

        await startSavingEvent( formValues );
        closeDateModal();
        setFormSubmitted(false);
    }



  return (
    <Modal
        isOpen={ isDateModalOpen }
        onRequestClose={ onCloseModal }
        style={ customStyles }
        className="modal"
        overlayClassName="modal-fondo"
        closeTimeoutMS={ 200 }
    >
        <h1> { formValues.id ? 'Editar evento' : 'Nuevo evento' } </h1>
        <hr />
        <form className="container" onSubmit={ onSubmit }>

            <div className="form-group mb-2">
                <label>Schedule</label>
                <select
                    className={ `form-control ${ scheduleClass }`}
                    value={ formValues.schedule_id }
                    onChange={ onScheduleChanged }
                >
                    <option value="">-- Seleccione --</option>
                    { schedules.map( schedule => (
                        <option key={ schedule.id } value={ schedule.id }>{ schedule.name }</option>
                    ))}
                </select>
                <small className="form-text text-muted">El evento dispara esta programación en la fecha indicada</small>
            </div>

            <div className="form-group mb-2">
                <label>Fecha y hora</label>
                <DatePicker
                    selected={ formValues.start }
                    onChange={ onDateChanged }
                    className="form-control"
                    dateFormat="Pp"
                    showTimeSelect
                    locale="es"
                    timeCaption="Hora"
                />
            </div>

            <button
                type="submit"
                className="btn btn-outline-primary btn-block"
            >
                <i className="far fa-save"></i>
                <span> Guardar</span>
            </button>

        </form>
    </Modal>
  )
}
