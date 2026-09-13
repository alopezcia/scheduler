import { useLocation, useNavigate } from 'react-router-dom';
import { useAuthStore } from "../../hooks/useAuthStore"


export const Navbar = () => {

  const { startLogout, user } = useAuthStore();
  const navigate = useNavigate();
  const location = useLocation();

  const isMaintenance = location.pathname.startsWith('/scada');

  return (
    <div className="navbar navbar-dark bg-dark mb-4 px-4">
        <span className="navbar-brand">
            <i className="fas fa-calendar-alt"></i>
            &nbsp;
            { user.name }
        </span>

        <div className="d-flex gap-2">
            <button
              className="btn btn-outline-light"
              onClick={ () => navigate( isMaintenance ? '/' : '/scada' ) }
            >
                <i className={ `fas ${ isMaintenance ? 'fa-calendar-alt' : 'fa-tools' }` }></i>
                &nbsp;
                <span>{ isMaintenance ? 'Calendario' : 'Mantenimiento' }</span>
            </button>

            <button
              className="btn btn-outline-danger"
              onClick={ startLogout }
            >
                <i className="fas fa-sign-out-alt"></i>
                &nbsp;
                <span>Salir</span>
            </button>
        </div>
    </div>
  )
}
