-- Roles de usuario: consultor (solo lectura), planificador (además crea/modifica eventos y
-- configuración) y administrador (además pausa/reanuda el scheduler y cancela ejecuciones).
-- Los usuarios que ya existen pasan a administrador para no quedarse sin nadie que pueda
-- gestionar el scheduler; los nuevos entran como consultor (salvo el primero de todos).
ALTER TABLE users ADD COLUMN role TEXT NOT NULL DEFAULT 'consultor'
    CHECK (role IN ('consultor', 'planificador', 'administrador'));

UPDATE users SET role = 'administrador';
