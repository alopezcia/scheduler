import { generateCronDates, MAX_CRON_ENTRIES } from '../../src/helpers/generateCronDates';


describe('Pruebas en generateCronDates', () => {

    const startDate = new Date('2026-09-06T00:00:00');

    test('debe de generar la cantidad de fechas solicitada', () => {

        const dates = generateCronDates({
            expression: '0 9 * * 1-5',
            startDate,
            count: 5,
        });

        expect(dates.length).toBe(5);
        dates.forEach( date => expect( date instanceof Date ).toBeTruthy() );

    });

    test('las fechas generadas deben respetar el patrón crontab (hora 9, días de semana)', () => {

        const dates = generateCronDates({
            expression: '0 9 * * 1-5',
            startDate,
            count: 3,
        });

        dates.forEach( date => {
            const day = date.getUTCDay();
            expect( day ).toBeGreaterThanOrEqual(1);
            expect( day ).toBeLessThanOrEqual(5);
        });

    });

    test('debe de lanzar un error si la expresión crontab es inválida', () => {

        expect(() => generateCronDates({
            expression: 'no-es-un-cron',
            startDate,
            count: 3,
        })).toThrow('La expresión crontab no es válida');

    });

    test('debe de lanzar un error si count no es un entero positivo', () => {

        expect(() => generateCronDates({
            expression: '0 9 * * *',
            startDate,
            count: 0,
        })).toThrow();

        expect(() => generateCronDates({
            expression: '0 9 * * *',
            startDate,
            count: -1,
        })).toThrow();

    });

    test('debe de lanzar un error si count excede el máximo permitido', () => {

        expect(() => generateCronDates({
            expression: '0 9 * * *',
            startDate,
            count: MAX_CRON_ENTRIES + 1,
        })).toThrow();

    });

});
