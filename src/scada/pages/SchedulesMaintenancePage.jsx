import { useEffect, useState } from 'react';
import Swal from 'sweetalert2';
import 'sweetalert2/dist/sweetalert2.min.css';

import { calendarApi } from '../../api';
import { generateCronDates, MAX_CRON_ENTRIES } from '../../helpers';
import { EntityMaintenance } from '../components/EntityMaintenance';

const TRIGGER_TYPE_OPTIONS = [
    { value: 'once', label: 'Una vez' },
    { value: 'cron', label: 'Cron' },
];

const DEFAULT_OCCURRENCES = 5;

const parseJson = ( text, fieldLabel ) => {
    try {
        return JSON.parse( text );
    } catch {
        throw new Error(`El campo ${ fieldLabel } debe ser un JSON válido (ej. true, 42, "auto")`);
    }
}

/**
 * Un schedule ya no referencia un event: es al revés, el event se genera a
 * partir del schedule. Al crear uno se materializa automáticamente su(s)
 * ocurrencia(s) como events en el calendario, en vez de exigir elegir un
 * event ya existente.
 */
const generateEventsForSchedule = async( schedule, occurrenceCount ) => {
    try {
        if ( schedule.trigger_type === 'once' ) {
            await calendarApi.post('/events', { schedule_id: schedule.id, start: schedule.start_date });
            return;
        }

        if ( schedule.trigger_type === 'cron' ) {
            const dates = generateCronDates({
                expression: schedule.cron_expr,
                startDate: schedule.start_date ? new Date( schedule.start_date ) : new Date(),
                count: occurrenceCount,
            });

            for ( const date of dates ) {
                await calendarApi.post('/events', { schedule_id: schedule.id, start: date.toISOString() });
            }
        }
    } catch (error) {
        console.log(error);
        Swal.fire(
            'Aviso',
            'La programación se guardó, pero no se pudieron generar todos sus eventos de calendario',
            'warning'
        );
    }
}

export const SchedulesMaintenancePage = () => {

    const [ tags, setTags ] = useState([]);

    useEffect(() => {
        calendarApi.get('/tags')
            .then(({ data }) => setTags( data.tags || [] ))
            .catch(() => setTags([]));
    }, []);

    const tagName = ( id ) => tags.find( tag => tag.id === id )?.name || id;

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
        { name: 'start_date', label: 'Fecha de inicio (ISO-8601)', type: 'text', placeholder: '2026-09-20T08:00:00.000Z', helpText: 'Obligatorio si el disparador es "once"; también se usa como referencia para el disparador "cron"' },
        { name: 'end_date', label: 'Fecha de fin (ISO-8601)', type: 'text', placeholder: '2026-09-20T08:00:00.000Z' },
        {
            name: 'occurrence_count', label: 'Ocurrencias a generar como events', type: 'number',
            helpText: `Solo aplica al crear un schedule "cron": cuántos events se generan de una vez en el calendario (máx. ${ MAX_CRON_ENTRIES })`,
        },
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
                start_date: '',
                end_date: '',
                occurrence_count: DEFAULT_OCCURRENCES,
                enabled: true,
                requires_confirmation: false,
            }) }
            toFormValues={ ( item ) => ({
                tag_id: item.tag_id,
                name: item.name,
                target_value: JSON.stringify( item.target_value ),
                trigger_type: item.trigger_type,
                cron_expr: item.cron_expr || '',
                start_date: item.start_date || '',
                end_date: item.end_date || '',
                occurrence_count: DEFAULT_OCCURRENCES,
                enabled: item.enabled,
                requires_confirmation: item.requires_confirmation,
            }) }
            toPayload={ ( form ) => ({
                name: form.name.trim(),
                tag_id: form.tag_id,
                target_value: parseJson( form.target_value, 'target_value' ),
                trigger_type: form.trigger_type,
                cron_expr: form.trigger_type === 'cron' ? form.cron_expr.trim() : null,
                start_date: form.start_date.trim() === '' ? null : form.start_date.trim(),
                end_date: form.end_date.trim() === '' ? null : form.end_date.trim(),
                enabled: !!form.enabled,
                requires_confirmation: !!form.requires_confirmation,
            }) }
            onSaved={ ({ item, formValues, isCreate }) => {
                if ( !isCreate ) return;

                const occurrenceCount = Math.min(
                    Math.max( Number( formValues.occurrence_count ) || DEFAULT_OCCURRENCES, 1 ),
                    MAX_CRON_ENTRIES
                );

                return generateEventsForSchedule( item, occurrenceCount );
            } }
        />
    )
}
