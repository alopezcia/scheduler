import { useSchedulerStore } from '../../hooks/useSchedulerStore';

const EXECUTION_LABELS = {
    started: { text: 'En curso', className: 'bg-primary' },
    ack: { text: 'Confirmada', className: 'bg-success' },
    sent: { text: 'Enviada', className: 'bg-success' },
    failed: { text: 'Fallida', className: 'bg-danger' },
    timeout: { text: 'Timeout', className: 'bg-warning text-dark' },
    cancelled: { text: 'Cancelada', className: 'bg-secondary' },
};

const formatTime = ( iso ) => new Date( iso ).toLocaleTimeString();

const connectionBadge = ({ socket, linked }) => {
    if ( socket === 'unauthorized' ) {
        return { text: 'Sesión caducada: vuelve a iniciar sesión', className: 'bg-danger' };
    }
    if ( socket !== 'open' ) {
        return { text: socket === 'connecting' ? 'Conectando con el servidor…' : 'Sin conexión con el servidor', className: 'bg-secondary' };
    }
    if ( linked === null ) return { text: 'Consultando…', className: 'bg-secondary' };
    return linked
        ? { text: 'Scheduler conectado', className: 'bg-success' }
        : { text: 'Scheduler no disponible', className: 'bg-danger' };
}

export const SchedulerPage = () => {

    const { canControl, linked, log, paused, running, socket, cancelExecution, pause, resume } = useSchedulerStore({ autoConnect: true });

    const badge = connectionBadge({ socket, linked });

    return (
        <>
            <div className="d-flex align-items-center gap-3 mb-4">
                <span className={ `badge ${ badge.className }` }>{ badge.text }</span>

                { canControl && (
                    <span className={ `badge ${ paused ? 'bg-warning text-dark' : 'bg-success' }` }>
                        { paused ? 'En pausa' : 'En marcha' }
                    </span>
                )}

                <div className="ms-auto btn-group">
                    <button
                        type="button"
                        className="btn btn-outline-success"
                        disabled={ !canControl || !paused }
                        onClick={ resume }
                    >
                        <i className="fas fa-play"></i> Iniciar
                    </button>
                    <button
                        type="button"
                        className="btn btn-outline-warning"
                        disabled={ !canControl || paused }
                        onClick={ pause }
                    >
                        <i className="fas fa-pause"></i> Pausar
                    </button>
                </div>
            </div>

            { paused && canControl && (
                <div className="alert alert-warning">
                    El scheduler no recoge tareas nuevas. Las ejecuciones en curso continúan; los eventos
                    que venzan durante la pausa y superen la tolerancia se registrarán como <code>timeout</code>.
                </div>
            )}

            <h5>Ejecuciones en curso</h5>
            <table className="table table-sm mb-4">
                <thead>
                    <tr>
                        <th>Schedule</th>
                        <th>Tag</th>
                        <th>Inicio</th>
                        <th></th>
                    </tr>
                </thead>
                <tbody>
                    { running.length === 0 && (
                        <tr><td colSpan="4" className="text-muted">No hay ejecuciones en curso</td></tr>
                    )}
                    { running.map( item => (
                        <tr key={ item.execution_id }>
                            <td>{ item.schedule_name }</td>
                            <td>{ item.tag_name }</td>
                            <td>{ formatTime( item.started_at ) }</td>
                            <td className="text-end">
                                <button
                                    type="button"
                                    className="btn btn-sm btn-outline-danger"
                                    disabled={ !canControl }
                                    onClick={ () => cancelExecution( item.execution_id ) }
                                >
                                    <i className="fas fa-ban"></i> Cancelar
                                </button>
                            </td>
                        </tr>
                    ))}
                </tbody>
            </table>

            <h5>Actividad reciente</h5>
            <table className="table table-sm">
                <thead>
                    <tr>
                        <th>Hora</th>
                        <th>Schedule</th>
                        <th>Tag</th>
                        <th>Estado</th>
                        <th>Detalle</th>
                    </tr>
                </thead>
                <tbody>
                    { log.length === 0 && (
                        <tr><td colSpan="5" className="text-muted">Sin actividad desde que abriste esta pantalla</td></tr>
                    )}
                    { log.map(( entry, index ) => {
                        const label = EXECUTION_LABELS[ entry.status ] || { text: entry.status, className: 'bg-secondary' };
                        return (
                            <tr key={ `${ entry.execution_id }-${ entry.status }-${ index }` }>
                                <td>{ formatTime( entry.at ) }</td>
                                <td>{ entry.schedule_name }</td>
                                <td>{ entry.tag_name }</td>
                                <td><span className={ `badge ${ label.className }` }>{ label.text }</span></td>
                                <td>{ entry.message }</td>
                            </tr>
                        );
                    })}
                </tbody>
            </table>
        </>
    )
}
