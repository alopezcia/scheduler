import { useState } from 'react';

import { Navbar } from '../../calendar';
import { useAuthStore } from '../../hooks';
import { AssetsMaintenancePage } from './AssetsMaintenancePage';
import { ConnectionsMaintenancePage } from './ConnectionsMaintenancePage';
import { SchedulerPage } from './SchedulerPage';
import { SchedulesMaintenancePage } from './SchedulesMaintenancePage';
import { SitesMaintenancePage } from './SitesMaintenancePage';
import { TagsMaintenancePage } from './TagsMaintenancePage';
import { UsersPage } from './UsersPage';

const TABS = [
    { key: 'sites', label: 'Sites', Component: SitesMaintenancePage },
    { key: 'assets', label: 'Assets', Component: AssetsMaintenancePage },
    { key: 'connections', label: 'Connections', Component: ConnectionsMaintenancePage },
    { key: 'tags', label: 'Tags', Component: TagsMaintenancePage },
    { key: 'schedules', label: 'Schedules', Component: SchedulesMaintenancePage },
    { key: 'scheduler', label: 'Scheduler', Component: SchedulerPage },
    { key: 'users', label: 'Usuarios', Component: UsersPage, adminOnly: true },
];

export const ScadaPage = () => {

    const { isAdmin } = useAuthStore();
    const [ activeTab, setActiveTab ] = useState( TABS[0].key );

    // La gestión de usuarios solo la ven los administradores (el backend también lo exige)
    const tabs = TABS.filter( tab => isAdmin || !tab.adminOnly );
    // Si pierde el permiso con la pestaña abierta, vuelve a la primera
    const current = tabs.find( tab => tab.key === activeTab ) ?? tabs[0];
    const ActiveComponent = current.Component;

    return (
        <>
            <Navbar />

            <div className="container-fluid px-4">
                <ul className="nav nav-pills mb-4">
                    { tabs.map( tab => (
                        <li className="nav-item" key={ tab.key }>
                            <button
                                type="button"
                                className={ `nav-link ${ current.key === tab.key ? 'active' : '' }` }
                                onClick={ () => setActiveTab( tab.key ) }
                            >
                                { tab.label }
                            </button>
                        </li>
                    ))}
                </ul>

                <ActiveComponent />
            </div>
        </>
    )
}
