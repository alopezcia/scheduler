import { render, screen } from '@testing-library/react';

import { ScadaPage } from '../../../src/scada/pages/ScadaPage';
import { useAuthStore } from '../../../src/hooks';

jest.mock('../../../src/hooks', () => ({ useAuthStore: jest.fn() }));
jest.mock('../../../src/calendar', () => ({ Navbar: () => null }));
jest.mock('../../../src/scada/pages/SitesMaintenancePage', () => ({ SitesMaintenancePage: () => null }));
jest.mock('../../../src/scada/pages/AssetsMaintenancePage', () => ({ AssetsMaintenancePage: () => null }));
jest.mock('../../../src/scada/pages/ConnectionsMaintenancePage', () => ({ ConnectionsMaintenancePage: () => null }));
jest.mock('../../../src/scada/pages/TagsMaintenancePage', () => ({ TagsMaintenancePage: () => null }));
jest.mock('../../../src/scada/pages/SchedulesMaintenancePage', () => ({ SchedulesMaintenancePage: () => null }));
jest.mock('../../../src/scada/pages/SchedulerPage', () => ({ SchedulerPage: () => null }));
jest.mock('../../../src/scada/pages/UsersPage', () => ({ UsersPage: () => null }));

describe('Pruebas en <ScadaPage />', () => {

    test('la pestaña Usuarios solo la ve un administrador', () => {
        useAuthStore.mockReturnValue({ isAdmin: true });
        const { unmount } = render(<ScadaPage />);
        expect( screen.queryByText('Usuarios') ).not.toBeNull();
        unmount();

        useAuthStore.mockReturnValue({ isAdmin: false });
        render(<ScadaPage />);
        expect( screen.queryByText('Usuarios') ).toBeNull();
        expect( screen.queryByText('Scheduler') ).not.toBeNull();
    });
});
