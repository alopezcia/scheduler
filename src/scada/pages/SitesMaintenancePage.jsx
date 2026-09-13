import { EntityMaintenance } from '../components/EntityMaintenance';

const fields = [
    { name: 'name', label: 'Nombre', type: 'text', required: true, placeholder: 'Planta Norte' },
];

const columns = [
    { key: 'name', label: 'Nombre' },
    { key: 'created_at', label: 'Creado', render: ( item ) => new Date( item.created_at ).toLocaleString() },
];

export const SitesMaintenancePage = () => {
    return (
        <EntityMaintenance
            title="Sites"
            endpoint="/sites"
            listKey="sites"
            columns={ columns }
            fields={ fields }
            getEmptyForm={ () => ({ name: '' }) }
            toFormValues={ ( item ) => ({ name: item.name }) }
            toPayload={ ( form ) => ({ name: form.name.trim() }) }
        />
    )
}
