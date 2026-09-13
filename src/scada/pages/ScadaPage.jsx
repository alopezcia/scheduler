import { useState } from 'react';

import { Navbar } from '../../calendar';
import { AssetsMaintenancePage } from './AssetsMaintenancePage';
import { ConnectionsMaintenancePage } from './ConnectionsMaintenancePage';
import { SchedulesMaintenancePage } from './SchedulesMaintenancePage';
import { SitesMaintenancePage } from './SitesMaintenancePage';
import { TagsMaintenancePage } from './TagsMaintenancePage';

const TABS = [
    { key: 'sites', label: 'Sites', Component: SitesMaintenancePage },
    { key: 'assets', label: 'Assets', Component: AssetsMaintenancePage },
    { key: 'connections', label: 'Connections', Component: ConnectionsMaintenancePage },
    { key: 'tags', label: 'Tags', Component: TagsMaintenancePage },
    { key: 'schedules', label: 'Schedules', Component: SchedulesMaintenancePage },
];

export const ScadaPage = () => {

    const [ activeTab, setActiveTab ] = useState( TABS[0].key );

    const ActiveComponent = TABS.find( tab => tab.key === activeTab ).Component;

    return (
        <>
            <Navbar />

            <div className="container-fluid px-4">
                <ul className="nav nav-pills mb-4">
                    { TABS.map( tab => (
                        <li className="nav-item" key={ tab.key }>
                            <button
                                type="button"
                                className={ `nav-link ${ activeTab === tab.key ? 'active' : '' }` }
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
