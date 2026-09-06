import { CronExpressionParser } from 'cron-parser';

export const MAX_CRON_ENTRIES = 100;

export const generateCronDates = ({ expression, startDate, count }) => {

    const entries = Number( count );

    if ( !Number.isInteger( entries ) || entries < 1 ) {
        throw new Error('El número de entradas debe ser un entero mayor a 0');
    }

    if ( entries > MAX_CRON_ENTRIES ) {
        throw new Error(`No se pueden generar más de ${ MAX_CRON_ENTRIES } entradas de una sola vez`);
    }

    let interval;

    try {
        interval = CronExpressionParser.parse( expression, { currentDate: startDate } );
    } catch (error) {
        throw new Error('La expresión crontab no es válida');
    }

    return interval.take( entries ).map( cronDate => cronDate.toDate() );
}
