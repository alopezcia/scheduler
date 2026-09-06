import { useUiStore } from '../../hooks';


export const FabAddCron = () => {

    const { openCronModal } = useUiStore();

    const handleClickCron = () => {
        openCronModal();
    }


  return (
    <button
        aria-label="btn-cron"
        className="btn btn-secondary fab-cron"
        onClick={ handleClickCron }
    >
        <i className="fas fa-business-time"></i>
    </button>
  )
}
