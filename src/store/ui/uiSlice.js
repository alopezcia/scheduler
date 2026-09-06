
import { createSlice } from '@reduxjs/toolkit';

export const uiSlice = createSlice({
    name: 'ui',
    initialState: {
        isDateModalOpen: false,
        isCronModalOpen: false,
    },
    reducers: {
        onOpenDateModal: ( state ) => {
            state.isDateModalOpen = true;
        },
        onCloseDateModal: ( state ) => {
            state.isDateModalOpen = false;
        },
        onOpenCronModal: ( state ) => {
            state.isCronModalOpen = true;
        },
        onCloseCronModal: ( state ) => {
            state.isCronModalOpen = false;
        },
    }
});


// Action creators are generated for each case reducer function
export const { onOpenDateModal, onCloseDateModal, onOpenCronModal, onCloseCronModal } = uiSlice.actions;

