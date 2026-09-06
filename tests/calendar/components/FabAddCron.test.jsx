import { fireEvent, render, screen } from '@testing-library/react';
import { FabAddCron } from '../../../src/calendar/components/FabAddCron';
import { useUiStore } from '../../../src/hooks/useUiStore';

jest.mock('../../../src/hooks/useUiStore');


describe('Pruebas en <FabAddCron />', () => {

    const mockOpenCronModal = jest.fn();

    beforeEach( () => jest.clearAllMocks() );

    test('debe de mostrar el componente correctamente', () => {

        useUiStore.mockReturnValue({
            openCronModal: mockOpenCronModal
        });

        render(<FabAddCron />);

        const btn = screen.getByLabelText('btn-cron');
        expect( btn.classList ).toContain('btn');
        expect( btn.classList ).toContain('fab-cron');

    });

    test('debe de llamar openCronModal al hacer click', () => {

        useUiStore.mockReturnValue({
            openCronModal: mockOpenCronModal
        });

        render(<FabAddCron />);

        const btn = screen.getByLabelText('btn-cron');
        fireEvent.click( btn );

        expect( mockOpenCronModal ).toHaveBeenCalledWith();

    });

});
