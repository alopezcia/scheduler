import { configureStore } from '@reduxjs/toolkit';
import { uiSlice, calendarSlice, authSlice, schedulerSlice } from './';


export const store = configureStore({
    reducer: {
        auth:     authSlice.reducer,
        calendar: calendarSlice.reducer,
        scheduler: schedulerSlice.reducer,
        ui:       uiSlice.reducer
    },
    middleware: (getDefaultMiddleware) => getDefaultMiddleware({
        serializableCheck: false
    })
})
