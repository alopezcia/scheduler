import { useEffect, useState } from 'react';

import { calendarApi } from '../../api';
import { EntityMaintenance } from '../components/EntityMaintenance';

const TRIGGER_TYPE_OPTIONS = [
    { value: 'once', label: 'Una vez' },
    { value: 'cron', label: 'Cron' },
    { value: 'calendar_event', label: 'Evento de calendario' },
];

const parseJson = ( text, fieldLabel ) => {
    try {
        return JSON.parse( text );
    } catch {
        throw new Error(`El campo ${ fieldLabel } debe ser un JSON válido (ej. true, 42, "auto")`);
    }
}

export const SchedulesMaintenancePage = () => {

    const [ tags, setTags ] = useState([]);
    const [ events, setEvents ] = useState([]);

    useEffect(() => {
        calendarApi.get('/tags')
            .then(({ data }) => setTags( data.tags || [] ))
            .catch(() => setTags([]));
        calendarApi.get('/events')
            .then(({ data }) => setEvents( data.eventos || [] ))
            .catch(() => setEvents([]));
    }, []);

    const tagName = ( id ) => tags.find( tag => tag.id === id )?.name || id;
    const eventTitle = ( id ) => events.find( event => event.id === id )?.title || id;

    const fields = [
        {
            name: 'tag_id', label: 'Tag', type: 'select', required: true,
            options: tags.map( tag => ({ value: tag.id, label: `${ tag.name } (${ tag.data_type })` }) ),
        },
        { name: 'name', label: 'Nombre', type: 'text', required: true, placeholder: 'Arranque turno mañana' },
        {
            name: 'target_value', label: 'Valor objetivo (JSON)', type: 'text', required: true,
            helpText: 'Debe respetar el data_type del tag: true/false, un número, o "texto" entre comillas si es enum',
        },
        { name: 'trigger_type', label: 'Disparador', type: 'select', required: true, options: TRIGGER_TYPE_OPTIONS },
        { name: 'cron_expr', label: 'Expresión cron', type: 'text', placeholder: '0 9 * * 1-5', helpText: 'Obligatorio si el disparador es cron' },
        {
            name: 'event_id', label: 'Evento de calendario', type: 'select',
            options: events.map( event => ({ value: event.id, label: event.title }) ),
            helpText: 'Obligatorio si el disparador es calendar_event',
        },
        { name: 'start_date', label: 'Fecha de inicio (ISO-8601)', type: 'text', placeholder: '2026-09-20T08:00:00.000Z', helpText: 'Obligatorio si el disparador es "once"' },
        { name: 'end_date', label: 'Fecha de fin (ISO-8601)', type: 'text', placeholder: '2026-09-20T08:00:00.000Z' },
        { name: 'enabled', label: 'Habilitada', type: 'checkbox' },
        { name: 'requires_confirmation', label: 'Requiere confirmación', type: 'checkbox' },
    ];

    const columns = [
        { key: 'name', label: 'Nombre' },
        { key: 'tag_id', label: 'Tag', render: ( item ) => tagName( item.tag_id ) },
        { key: 'target_value', label: 'Valor objetivo', render: ( item ) => JSON.stringify( item.target_value ) },
        { key: 'trigger_type', label: 'Disparador' },
        { key: 'enabled', label: 'Habilitada', render: ( item ) => item.enabled ? 'Sí' : 'No' },
    ];

    return (
        <EntityMaintenance
            title="Schedules"
            endpoint="/schedules"
            listKey="schedules"
            columns={ columns }
            fields={ fields }
            getEmptyForm={ () => ({
                tag_id: '',
                name: '',
                target_value: 'true',
                trigger_type: 'once',
                cron_expr: '',
                event_id: '',
                start_date: '',
                end_date: '',
                enabled: true,
                requires_confirmation: false,
            }) }
            toFormValues={ ( item ) => ({
                tag_id: item.tag_id,
                name: item.name,
                target_value: JSON.stringify( item.target_value ),
                trigger_type: item.trigger_type,
                cron_expr: item.cron_expr || '',
                event_id: item.event_id || '',
                start_date: item.start_date || '',
                end_date: item.end_date || '',
                enabled: item.enabled,
                requires_confirmation: item.requires_confirmation,
            }) }
            toPayload={ ( form ) => ({
                name: form.name.trim(),
                tag_id: form.tag_id,
                target_value: parseJson( form.target_value, 'target_value' ),
                trigger_type: form.trigger_type,
                cron_expr: form.trigger_type === 'cron' ? form.cron_expr.trim() : null,
                event_id: form.trigger_type === 'calendar_event' ? ( form.event_id || null ) : null,
                start_date: form.start_date.trim() === '' ? null : form.start_date.trim(),
                end_date: form.end_date.trim() === '' ? null : form.end_date.trim(),
                enabled: !!form.enabled,
                requires_confirmation: !!form.requires_confirmation,
            }) }
        />
    )
}
