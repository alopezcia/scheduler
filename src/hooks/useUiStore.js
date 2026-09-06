import { useDispatch, useSelector } from 'react-redux';
import { onCloseDateModal, onOpenDateModal, onCloseCronModal, onOpenCronModal } from '../store';


export const useUiStore = () => {

    const dispatch = useDispatch();

    const {
        isDateModalOpen,
        isCronModalOpen,
    } = useSelector( state => state.ui );

    const openDateModal = () => {
        dispatch( onOpenDateModal() )
    }

    const closeDateModal = () => {
        dispatch( onCloseDateModal() )
    }

    const toggleDateModal = () => {
        (isDateModalOpen)
            ? closeDateModal()
            : openDateModal();
    }

    const openCronModal = () => {
        dispatch( onOpenCronModal() )
    }

    const closeCronModal = () => {
        dispatch( onCloseCronModal() )
    }

    const toggleCronModal = () => {
        (isCronModalOpen)
            ? closeCronModal()
            : openCronModal();
    }

    return {
        //* Propiedades
        isDateModalOpen,
        isCronModalOpen,

        //* Métodos
        closeDateModal,
        openDateModal,
        toggleDateModal,
        closeCronModal,
        openCronModal,
        toggleCronModal,
    }

}