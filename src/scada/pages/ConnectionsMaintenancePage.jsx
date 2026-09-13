import { useEffect, useState } from 'react';

import { calendarApi } from '../../api';
import { EntityMaintenance } from '../components/EntityMaintenance';

const PROTOCOL_OPTIONS = [
    { value: 'opcua', label: 'OPC UA' },
    { value: 'mqtt', label: 'MQTT' },
    { value: 's7', label: 'S7' },
];

const parseJson = ( text, fieldLabel ) => {
    try {
        return JSON.parse( text );
    } catch {
        throw new Error(`El campo ${ fieldLabel } debe ser un JSON válido`);
    }
}

export const ConnectionsMaintenancePage = () => {

    const [ assets, setAssets ] = useState([]);

    useEffect(() => {
        calendarApi.get('/assets')
            .then(({ data }) => setAssets( data.assets || [] ))
            .catch(() => setAssets([]));
    }, []);

    const assetName = ( id ) => assets.find( asset => asset.id === id )?.name || id;

    const fields = [
        {
            name: 'asset_id', label: 'Asset', type: 'select', required: true,
            options: assets.map( asset => ({ value: asset.id, label: asset.name }) ),
        },
        { name: 'name', label: 'Nombre', type: 'text', required: true, placeholder: 'PLC1' },
        { name: 'protocol', label: 'Protocolo', type: 'select', required: true, options: PROTOCOL_OPTIONS },
        {
            name: 'config', label: 'Config (JSON)', type: 'textarea', required: true, rows: 5,
            helpText: 'opcua: { endpoint_url, security_policy, credentials_ref } · mqtt: { broker_url, base_topic, qos, tls } · s7: { ip, rack, slot }',
        },
    ];

    const columns = [
        { key: 'name', label: 'Nombre' },
        { key: 'protocol', label: 'Protocolo' },
        { key: 'status', label: 'Estado' },
        { key: 'asset_id', label: 'Asset', render: ( item ) => assetName( item.asset_id ) },
        { key: 'last_seen', label: 'Último visto', render: ( item ) => item.last_seen ? new Date( item.last_seen ).toLocaleString() : '—' },
    ];

    return (
        <EntityMaintenance
            title="Connections"
            endpoint="/connections"
            listKey="connections"
            columns={ columns }
            fields={ fields }
            getEmptyForm={ () => ({ asset_id: '', name: '', protocol: 'opcua', config: '{}' }) }
            toFormValues={ ( item ) => ({
                asset_id: item.asset_id,
                name: item.name,
                protocol: item.protocol,
                config: JSON.stringify( item.config, null, 2 ),
            }) }
            toPayload={ ( form ) => ({
                asset_id: form.asset_id,
                name: form.name.trim(),
                protocol: form.protocol,
                config: parseJson( form.config, 'config' ),
            }) }
        />
    )
}
