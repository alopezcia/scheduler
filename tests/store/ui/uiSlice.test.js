import { onCloseDateModal, onOpenDateModal, onCloseCronModal, onOpenCronModal, uiSlice } from "../../../src/store/ui/uiSlice";


describe('Pruebas en uiSlice', () => {

    test('debe de regresar el estado por defecto', () => {

        expect(uiSlice.getInitialState()).toEqual({ isDateModalOpen: false, isCronModalOpen: false })

    });

    test('debe de cambiar el isDateModalOpen correctamente', () => {

        let state = uiSlice.getInitialState();
        state = uiSlice.reducer( state, onOpenDateModal() )
        expect(state.isDateModalOpen).toBeTruthy();

        state = uiSlice.reducer( state, onCloseDateModal() );
        expect(state.isDateModalOpen).toBeFalsy();


    });

    test('debe de cambiar el isCronModalOpen correctamente', () => {

        let state = uiSlice.getInitialState();
        state = uiSlice.reducer( state, onOpenCronModal() )
        expect(state.isCronModalOpen).toBeTruthy();

        state = uiSlice.reducer( state, onCloseCronModal() );
        expect(state.isCronModalOpen).toBeFalsy();

    });


});