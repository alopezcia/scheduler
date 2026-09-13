import { useEffect, useState } from 'react';

import { calendarApi } from '../../api';
import { EntityMaintenance } from '../components/EntityMaintenance';

const TAG_KIND_OPTIONS = [
    { value: 'telemando', label: 'Telemando' },
    { value: 'consigna', label: 'Consigna' },
    { value: 'medida', label: 'Medida' },
];

const DATA_TYPE_OPTIONS = [
    { value: 'bool', label: 'Bool' },
    { value: 'int', label: 'Int' },
    { value: 'float', label: 'Float' },
    { value: 'enum', label: 'Enum' },
];

const READ_WRITE_OPTIONS = [
    { value: 'read', label: 'Lectura' },
    { value: 'write', label: 'Escritura' },
    { value: 'read_write', label: 'Lectura/Escritura' },
];

const parseJson = ( text, fieldLabel ) => {
    try {
        return JSON.parse( text );
    } catch {
        throw new Error(`El campo ${ fieldLabel } debe ser un JSON válido`);
    }
}

export const TagsMaintenancePage = () => {

    const [ connections, setConnections ] = useState([]);
    const [ assets, setAssets ] = useState([]);

    useEffect(() => {
        calendarApi.get('/connections')
            .then(({ data }) => setConnections( data.connections || [] ))
            .catch(() => setConnections([]));
        calendarApi.get('/assets')
            .then(({ data }) => setAssets( data.assets || [] ))
            .catch(() => setAssets([]));
    }, []);

    const connectionLabel = ( id ) => {
        const connection = connections.find( c => c.id === id );
        return connection ? `${ connection.name } (${ connection.protocol })` : id;
    }
    const assetName = ( id ) => assets.find( asset => asset.id === id )?.name || id;

    const fields = [
        {
            name: 'connection_id', label: 'Connection', type: 'select', required: true,
            options: connections.map( connection => ({ value: connection.id, label: `${ connection.name } (${ connection.protocol })` }) ),
            helpText: 'El asset del tag se toma automáticamente del asset de esta conexión.',
        },
        { name: 'name', label: 'Nombre', type: 'text', required: true, placeholder: 'Motor1_Run' },
        { name: 'description', label: 'Descripción', type: 'text' },
        { name: 'tag_kind', label: 'Naturaleza', type: 'select', required: true, options: TAG_KIND_OPTIONS },
        { name: 'data_type', label: 'Tipo de dato', type: 'select', required: true, options: DATA_TYPE_OPTIONS },
        { name: 'unit', label: 'Unidad', type: 'text', placeholder: 'kW, °C, bar…' },
        {
            name: 'address', label: 'Address (JSON)', type: 'textarea', required: true, rows: 3,
            helpText: 'opcua: { node_id } · mqtt: { topic, json_pointer } · s7: { db_number, offset, bit, s7_type }',
        },
        { name: 'read_write', label: 'Acceso', type: 'select', required: true, options: READ_WRITE_OPTIONS },
        { name: 'min_value', label: 'Valor mínimo', type: 'number' },
        { name: 'max_value', label: 'Valor máximo', type: 'number' },
        {
            name: 'allowed_values', label: 'Valores permitidos (JSON)', type: 'textarea', rows: 2,
            helpText: 'Array JSON, obligatorio si el tipo de dato es enum. Ej: ["auto", "manual"]',
        },
        { name: 'requires_sbo', label: 'Requiere Select-Before-Operate', type: 'checkbox' },
        { name: 'requires_ack', label: 'Requiere confirmación (ack)', type: 'checkbox' },
    ];

    const columns = [
        { key: 'name', label: 'Nombre' },
        { key: 'tag_kind', label: 'Naturaleza' },
        { key: 'data_type', label: 'Tipo' },
        { key: 'connection_id', label: 'Connection', render: ( item ) => connectionLabel( item.connection_id ) },
        { key: 'asset_id', label: 'Asset', render: ( item ) => assetName( item.asset_id ) },
        { key: 'read_write', label: 'Acceso' },
    ];

    return (
        <EntityMaintenance
            title="Tags"
            endpoint="/tags"
            listKey="tags"
            columns={ columns }
            fields={ fields }
            getEmptyForm={ () => ({
                connection_id: '',
                name: '',
                description: '',
                tag_kind: 'medida',
                data_type: 'bool',
                unit: '',
                address: '{}',
                read_write: 'write',
                min_value: '',
                max_value: '',
                allowed_values: '',
                requires_sbo: false,
                requires_ack: false,
            }) }
            toFormValues={ ( item ) => ({
                connection_id: item.connection_id,
                name: item.name,
                description: item.description || '',
                tag_kind: item.tag_kind,
                data_type: item.data_type,
                unit: item.unit || '',
                address: JSON.stringify( item.address, null, 2 ),
                read_write: item.read_write,
                min_value: item.min_value ?? '',
                max_value: item.max_value ?? '',
                allowed_values: item.allowed_values ? JSON.stringify( item.allowed_values ) : '',
                requires_sbo: item.requires_sbo,
                requires_ack: item.requires_ack,
            }) }
            toPayload={ ( form ) => {
                const connection = connections.find( c => c.id === form.connection_id );
                if ( !connection ) {
                    throw new Error('Selecciona una connection válida');
                }

                return {
                    connection_id: form.connection_id,
                    asset_id: connection.asset_id,
                    name: form.name.trim(),
                    description: form.description || '',
                    tag_kind: form.tag_kind,
                    data_type: form.data_type,
                    unit: form.unit || null,
                    address: parseJson( form.address, 'address' ),
                    read_write: form.read_write,
                    min_value: form.min_value === '' ? null : Number( form.min_value ),
                    max_value: form.max_value === '' ? null : Number( form.max_value ),
                    allowed_values: form.allowed_values.trim() === '' ? null : parseJson( form.allowed_values, 'allowed_values' ),
                    requires_sbo: !!form.requires_sbo,
                    requires_ack: !!form.requires_ack,
                };
            }}
        />
    )
}
