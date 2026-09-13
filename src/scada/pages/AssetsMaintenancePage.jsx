import { useEffect, useState } from 'react';

import { calendarApi } from '../../api';
import { EntityMaintenance } from '../components/EntityMaintenance';

export const AssetsMaintenancePage = () => {

    const [ sites, setSites ] = useState([]);
    const [ assets, setAssets ] = useState([]);

    useEffect(() => {
        calendarApi.get('/sites')
            .then(({ data }) => setSites( data.sites || [] ))
            .catch(() => setSites([]));
    }, []);

    const siteName = ( id ) => sites.find( site => site.id === id )?.name || id;
    const assetName = ( id ) => assets.find( asset => asset.id === id )?.name || '—';

    const fields = [
        {
            name: 'site_id', label: 'Site', type: 'select', required: true,
            options: sites.map( site => ({ value: site.id, label: site.name }) ),
        },
        {
            name: 'parent_asset_id', label: 'Asset padre', type: 'select',
            options: assets.map( asset => ({ value: asset.id, label: asset.name }) ),
            helpText: 'Opcional: déjalo vacío para un asset de nivel raíz. Debe pertenecer al mismo site.',
        },
        { name: 'name', label: 'Nombre', type: 'text', required: true, placeholder: 'Área 1' },
        { name: 'kind', label: 'Tipo', type: 'text', required: true, placeholder: 'plant, area, equipment, plc…' },
    ];

    const columns = [
        { key: 'name', label: 'Nombre' },
        { key: 'kind', label: 'Tipo' },
        { key: 'site_id', label: 'Site', render: ( item ) => siteName( item.site_id ) },
        { key: 'parent_asset_id', label: 'Padre', render: ( item ) => item.parent_asset_id ? assetName( item.parent_asset_id ) : '—' },
    ];

    return (
        <EntityMaintenance
            title="Assets"
            endpoint="/assets"
            listKey="assets"
            columns={ columns }
            fields={ fields }
            onItemsChange={ setAssets }
            getEmptyForm={ () => ({ site_id: '', parent_asset_id: '', name: '', kind: '' }) }
            toFormValues={ ( item ) => ({
                site_id: item.site_id,
                parent_asset_id: item.parent_asset_id || '',
                name: item.name,
                kind: item.kind,
            }) }
            toPayload={ ( form ) => ({
                site_id: form.site_id,
                parent_asset_id: form.parent_asset_id || null,
                name: form.name.trim(),
                kind: form.kind.trim(),
            }) }
        />
    )
}
