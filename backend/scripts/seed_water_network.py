#!/usr/bin/env python3
"""
Carga calendar.db con datos de prueba inventados de una red de distribución
de agua potable: ETAP, depósito regulador, estación de bombeo y un sector de
red con válvulas reguladoras de presión.

Uso:
    python scripts/seed_water_network.py [ruta_a_calendar.db]

Por defecto usa calendar.db en el directorio actual. El script es idempotente
respecto al usuario (reutiliza el primero que encuentre en `users`, o crea uno
"Demo SCADA" si la tabla está vacía) pero SIEMPRE inserta un conjunto nuevo de
sites/assets/connections/tags/schedules/interlocks/events con ids nuevos, así
que se puede ejecutar varias veces sin pisar los datos existentes.
"""

import json
import sqlite3
import sys
import uuid
from datetime import datetime, timedelta, timezone

DB_PATH = sys.argv[1] if len(sys.argv) > 1 else "calendar.db"


def new_id():
    return str(uuid.uuid4())


def now_iso(offset_days=0, hour=None, minute=0):
    dt = datetime.now(timezone.utc) + timedelta(days=offset_days)
    if hour is not None:
        dt = dt.replace(hour=hour, minute=minute, second=0, microsecond=0)
    return dt.strftime("%Y-%m-%dT%H:%M:%S.000Z")


def main():
    con = sqlite3.connect(DB_PATH)
    con.execute("PRAGMA foreign_keys = ON")
    cur = con.cursor()

    # --- Usuario propietario de los schedules/events de prueba -------------
    cur.execute("SELECT id, name, email FROM users ORDER BY created_at LIMIT 1")
    row = cur.fetchone()
    if row:
        user_id = row[0]
        print(f"Usando usuario existente: {row[1]} <{row[2]}>")
    else:
        user_id = new_id()
        cur.execute(
            "INSERT INTO users (id, name, email, password_hash) VALUES (?, ?, ?, ?)",
            (user_id, "Demo SCADA", "demo.scada@example.com", "!"),
        )
        print("No había usuarios: creado 'Demo SCADA' <demo.scada@example.com>")

    ids = {}  # nombre lógico -> id, para referenciar entre tablas

    def add_site(key, name):
        sid = new_id()
        ids[key] = sid
        cur.execute("INSERT INTO sites (id, name) VALUES (?, ?)", (sid, name))
        return sid

    def add_asset(key, site_key, name, kind, parent_key=None):
        aid = new_id()
        ids[key] = aid
        cur.execute(
            "INSERT INTO assets (id, site_id, parent_asset_id, name, kind) VALUES (?, ?, ?, ?, ?)",
            (aid, ids[site_key], ids[parent_key] if parent_key else None, name, kind),
        )
        return aid

    def add_connection(key, asset_key, name, protocol, config):
        cid = new_id()
        ids[key] = cid
        cur.execute(
            "INSERT INTO connections (id, asset_id, name, protocol, config, status, last_seen) "
            "VALUES (?, ?, ?, ?, ?, ?, ?)",
            (
                cid,
                ids[asset_key],
                name,
                protocol,
                json.dumps(config, ensure_ascii=False),
                "online",
                now_iso(),
            ),
        )
        return cid

    def add_tag(
        key,
        connection_key,
        asset_key,
        name,
        tag_kind,
        data_type,
        address,
        description="",
        unit=None,
        read_write="write",
        min_value=None,
        max_value=None,
        allowed_values=None,
        requires_sbo=False,
        requires_ack=False,
    ):
        tid = new_id()
        ids[key] = tid
        cur.execute(
            """INSERT INTO tags
               (id, connection_id, asset_id, name, description, tag_kind, data_type, unit,
                address, read_write, min_value, max_value, allowed_values, requires_sbo, requires_ack)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)""",
            (
                tid,
                ids[connection_key],
                ids[asset_key],
                name,
                description,
                tag_kind,
                data_type,
                unit,
                json.dumps(address, ensure_ascii=False),
                read_write,
                min_value,
                max_value,
                json.dumps(allowed_values, ensure_ascii=False) if allowed_values else None,
                int(requires_sbo),
                int(requires_ack),
            ),
        )
        return tid

    def add_schedule(
        key,
        tag_key,
        name,
        target_value,
        trigger_type,
        cron_expr=None,
        start_date=None,
        end_date=None,
        enabled=True,
        requires_confirmation=False,
    ):
        sid = new_id()
        ids[key] = sid
        cur.execute(
            """INSERT INTO schedules
               (id, name, tag_id, target_value, trigger_type, cron_expr, start_date, end_date,
                enabled, requires_confirmation, created_by)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)""",
            (
                sid,
                name,
                ids[tag_key],
                json.dumps(target_value),
                trigger_type,
                cron_expr,
                start_date,
                end_date,
                int(enabled),
                int(requires_confirmation),
                user_id,
            ),
        )
        return sid

    def add_interlock(tag_key, condition_tag_key, operator, condition_value, action="block"):
        iid = new_id()
        cur.execute(
            """INSERT INTO interlocks
               (id, tag_id, condition_tag_id, operator, condition_value, action)
               VALUES (?, ?, ?, ?, ?, ?)""",
            (
                iid,
                ids[tag_key],
                ids[condition_tag_key],
                operator,
                json.dumps(condition_value),
                action,
            ),
        )
        return iid

    def add_event(schedule_key, start_date):
        eid = new_id()
        cur.execute(
            "INSERT INTO events (id, schedule_id, start_date, user_id) VALUES (?, ?, ?, ?)",
            (eid, ids[schedule_key], start_date, user_id),
        )
        return eid

    # =========================================================================
    # Site 1: ETAP (planta potabilizadora)
    # =========================================================================
    add_site("etap", "ETAP Vega del Río")
    add_asset("etap_captacion", "etap", "Área de Captación", "area")
    add_asset("etap_bomba_capt1", "etap", "Bomba Captación 1", "pump", "etap_captacion")
    add_asset("etap_bomba_capt2", "etap", "Bomba Captación 2", "pump", "etap_captacion")
    add_asset("etap_filtracion", "etap", "Área de Filtración", "area")
    add_asset("etap_filtro1", "etap", "Filtro de Arena 1", "filter", "etap_filtracion")
    add_asset("etap_filtro2", "etap", "Filtro de Arena 2", "filter", "etap_filtracion")
    add_asset("etap_cloracion", "etap", "Área de Cloración", "area")
    add_asset("etap_dosificador", "etap", "Dosificador de Cloro", "doser", "etap_cloracion")
    add_asset("etap_salida", "etap", "Salida de Planta", "metering")

    add_connection(
        "conn_etap_captacion",
        "etap_captacion",
        "RTU-ETAP-Captacion",
        "s7",
        {"ip": "10.20.1.10", "rack": 0, "slot": 1},
    )
    add_connection(
        "conn_etap_proceso",
        "etap_filtracion",
        "RTU-ETAP-Proceso",
        "opcua",
        {
            "endpoint_url": "opc.tcp://10.20.1.20:4840",
            "security_policy": "http://opcfoundation.org/UA/SecurityPolicy#Basic256Sha256",
            "credentials_ref": "cred-etap-proceso",
        },
    )

    add_tag(
        "tag_bomba_capt1_marcha", "conn_etap_captacion", "etap_bomba_capt1",
        "Bomba1_Captacion_Marcha", "telemando", "bool",
        {"db_number": 10, "offset": 0, "bit": 0, "s7_type": "bool"},
        description="Arranque/paro bomba de captación 1", requires_sbo=True, requires_ack=True,
    )
    add_tag(
        "tag_bomba_capt2_marcha", "conn_etap_captacion", "etap_bomba_capt2",
        "Bomba2_Captacion_Marcha", "telemando", "bool",
        {"db_number": 10, "offset": 0, "bit": 1, "s7_type": "bool"},
        description="Arranque/paro bomba de captación 2 (reserva)", requires_sbo=True, requires_ack=True,
    )
    add_tag(
        "tag_nivel_pozo", "conn_etap_captacion", "etap_captacion",
        "Nivel_Pozo_Captacion", "medida", "float",
        {"db_number": 10, "offset": 4, "bit": 0, "s7_type": "real"},
        description="Nivel del pozo de captación", unit="%", read_write="read",
        min_value=0, max_value=100,
    )
    add_tag(
        "tag_filtro1_perdida", "conn_etap_proceso", "etap_filtro1",
        "Filtro1_Perdida_Carga", "medida", "float", {"node_id": "ns=2;s=Filtro1.PerdidaCarga"},
        description="Pérdida de carga filtro de arena 1", unit="bar", read_write="read",
        min_value=0, max_value=2,
    )
    add_tag(
        "tag_filtro2_perdida", "conn_etap_proceso", "etap_filtro2",
        "Filtro2_Perdida_Carga", "medida", "float", {"node_id": "ns=2;s=Filtro2.PerdidaCarga"},
        description="Pérdida de carga filtro de arena 2", unit="bar", read_write="read",
        min_value=0, max_value=2,
    )
    add_tag(
        "tag_turbidez", "conn_etap_proceso", "etap_filtracion",
        "Turbidez_Salida_Filtros", "medida", "float", {"node_id": "ns=2;s=Filtracion.Turbidez"},
        description="Turbidez a la salida de filtros", unit="NTU", read_write="read",
        min_value=0, max_value=50,
    )
    add_tag(
        "tag_dosificador_consigna", "conn_etap_proceso", "etap_dosificador",
        "Dosificador_Consigna_Cloro", "consigna", "float", {"node_id": "ns=2;s=Dosificador.Consigna"},
        description="Consigna de dosificación de cloro", unit="mg/L", read_write="read_write",
        min_value=0.2, max_value=3,
    )
    add_tag(
        "tag_dosificador_marcha", "conn_etap_proceso", "etap_dosificador",
        "Dosificador_Marcha", "telemando", "bool", {"node_id": "ns=2;s=Dosificador.Marcha"},
        description="Marcha de la bomba dosificadora de cloro", requires_ack=True,
    )
    add_tag(
        "tag_cloro_residual", "conn_etap_proceso", "etap_salida",
        "Cloro_Residual_Salida", "medida", "float", {"node_id": "ns=2;s=Salida.CloroResidual"},
        description="Cloro residual libre a la salida de planta", unit="mg/L", read_write="read",
        min_value=0, max_value=5,
    )
    add_tag(
        "tag_caudal_salida_etap", "conn_etap_proceso", "etap_salida",
        "Caudal_Salida_ETAP", "medida", "float", {"node_id": "ns=2;s=Salida.Caudal"},
        description="Caudal de salida de la planta potabilizadora", unit="m3/h", read_write="read",
        min_value=0, max_value=600,
    )

    # =========================================================================
    # Site 2: Depósito regulador
    # =========================================================================
    add_site("deposito", "Depósito Regulador Collado Alto")
    add_asset("dep_caseta", "deposito", "Caseta de Válvulas", "valve_house")
    add_asset("dep1", "deposito", "Depósito 1 (2.500 m³)", "tank")
    add_asset("dep2", "deposito", "Depósito 2 (2.500 m³)", "tank")
    add_asset("dep_valv_ent1", "deposito", "Válvula Entrada Depósito 1", "valve", "dep_caseta")
    add_asset("dep_valv_sal1", "deposito", "Válvula Salida Depósito 1", "valve", "dep_caseta")
    add_asset("dep_valv_ent2", "deposito", "Válvula Entrada Depósito 2", "valve", "dep_caseta")
    add_asset("dep_valv_sal2", "deposito", "Válvula Salida Depósito 2", "valve", "dep_caseta")

    add_connection(
        "conn_deposito",
        "dep_caseta",
        "RTU-Deposito-Collado",
        "mqtt",
        {"broker_url": "mqtts://mqtt.aguapotable.local:8883", "base_topic": "scada/deposito-collado", "qos": 1, "tls": True},
    )

    add_tag(
        "tag_nivel_dep1", "conn_deposito", "dep1", "Nivel_Deposito1", "medida", "float",
        {"topic": "scada/deposito-collado/dep1/nivel", "json_pointer": "/value"},
        description="Nivel de lámina de agua del depósito 1", unit="%", read_write="read",
        min_value=0, max_value=100,
    )
    add_tag(
        "tag_nivel_dep1_consigna", "conn_deposito", "dep1", "Nivel_Deposito1_Consigna", "consigna", "float",
        {"topic": "scada/deposito-collado/dep1/consigna", "json_pointer": "/value"},
        description="Nivel objetivo de llenado del depósito 1", unit="%", read_write="read_write",
        min_value=20, max_value=95,
    )
    add_tag(
        "tag_nivel_dep2", "conn_deposito", "dep2", "Nivel_Deposito2", "medida", "float",
        {"topic": "scada/deposito-collado/dep2/nivel", "json_pointer": "/value"},
        description="Nivel de lámina de agua del depósito 2", unit="%", read_write="read",
        min_value=0, max_value=100,
    )
    add_tag(
        "tag_valv_ent1_cmd", "conn_deposito", "dep_valv_ent1", "ValvEntrada_Dep1_Cmd", "telemando", "enum",
        {"topic": "scada/deposito-collado/valv-ent1/cmd", "json_pointer": "/cmd"},
        description="Orden de apertura/cierre válvula de entrada depósito 1",
        allowed_values=["abrir", "cerrar"], requires_ack=True,
    )
    add_tag(
        "tag_valv_ent1_estado", "conn_deposito", "dep_valv_ent1", "ValvEntrada_Dep1_Estado", "medida", "enum",
        {"topic": "scada/deposito-collado/valv-ent1/estado", "json_pointer": "/estado"},
        description="Estado real de la válvula de entrada depósito 1", read_write="read",
        allowed_values=["abierta", "cerrada", "en_transito"],
    )
    add_tag(
        "tag_valv_sal1_cmd", "conn_deposito", "dep_valv_sal1", "ValvSalida_Dep1_Cmd", "telemando", "bool",
        {"topic": "scada/deposito-collado/valv-sal1/cmd", "json_pointer": "/cmd"},
        description="Orden de apertura/cierre válvula de salida depósito 1", requires_ack=True,
    )
    add_tag(
        "tag_valv_ent2_cmd", "conn_deposito", "dep_valv_ent2", "ValvEntrada_Dep2_Cmd", "telemando", "enum",
        {"topic": "scada/deposito-collado/valv-ent2/cmd", "json_pointer": "/cmd"},
        description="Orden de apertura/cierre válvula de entrada depósito 2",
        allowed_values=["abrir", "cerrar"], requires_ack=True,
    )
    add_tag(
        "tag_valv_sal2_cmd", "conn_deposito", "dep_valv_sal2", "ValvSalida_Dep2_Cmd", "telemando", "bool",
        {"topic": "scada/deposito-collado/valv-sal2/cmd", "json_pointer": "/cmd"},
        description="Orden de apertura/cierre válvula de salida depósito 2", requires_ack=True,
    )

    # =========================================================================
    # Site 3: Estación de bombeo
    # =========================================================================
    add_site("bombeo", "Estación de Bombeo Sector Norte")
    add_asset("bombeo_sala", "bombeo", "Sala de Bombas", "area")
    add_asset("bombeo_b1", "bombeo", "Bomba Impulsión 1", "pump", "bombeo_sala")
    add_asset("bombeo_b2", "bombeo", "Bomba Impulsión 2", "pump", "bombeo_sala")
    add_asset("bombeo_b3", "bombeo", "Bomba Impulsión 3 (Reserva)", "pump", "bombeo_sala")
    add_asset("bombeo_colector", "bombeo", "Colector de Impulsión", "header")

    add_connection(
        "conn_bombeo",
        "bombeo_sala",
        "RTU-Bombeo-Norte",
        "s7",
        {"ip": "10.20.2.10", "rack": 0, "slot": 2},
    )

    for n, key_prefix, asset_key in (
        (1, "b1", "bombeo_b1"),
        (2, "b2", "bombeo_b2"),
        (3, "b3", "bombeo_b3"),
    ):
        add_tag(
            f"tag_bombeo_{key_prefix}_marcha", "conn_bombeo", asset_key,
            f"Bomba{n}_Impulsion_Marcha", "telemando", "bool",
            {"db_number": 20, "offset": 0, "bit": n - 1, "s7_type": "bool"},
            description=f"Arranque/paro bomba de impulsión {n}", requires_sbo=True, requires_ack=True,
        )
        add_tag(
            f"tag_bombeo_{key_prefix}_velocidad", "conn_bombeo", asset_key,
            f"Bomba{n}_Velocidad_Consigna", "consigna", "float",
            {"db_number": 20, "offset": 10 + n * 4, "bit": 0, "s7_type": "real"},
            description=f"Consigna de velocidad variador bomba {n}", unit="%", read_write="read_write",
            min_value=0, max_value=100,
        )
        add_tag(
            f"tag_bombeo_{key_prefix}_horas", "conn_bombeo", asset_key,
            f"Bomba{n}_Horas_Funcionamiento", "medida", "float",
            {"db_number": 20, "offset": 30 + n * 4, "bit": 0, "s7_type": "real"},
            description=f"Horas acumuladas de funcionamiento bomba {n}", unit="h", read_write="read",
            min_value=0,
        )

    add_tag(
        "tag_presion_colector", "conn_bombeo", "bombeo_colector",
        "Presion_Colector", "medida", "float",
        {"db_number": 20, "offset": 60, "bit": 0, "s7_type": "real"},
        description="Presión en el colector de impulsión", unit="bar", read_write="read",
        min_value=0, max_value=10,
    )
    add_tag(
        "tag_presion_consigna_colector", "conn_bombeo", "bombeo_colector",
        "Presion_Consigna_Colector", "consigna", "float",
        {"db_number": 20, "offset": 64, "bit": 0, "s7_type": "real"},
        description="Consigna de presión de impulsión", unit="bar", read_write="read_write",
        min_value=2, max_value=8,
    )
    add_tag(
        "tag_caudal_impulsion", "conn_bombeo", "bombeo_colector",
        "Caudal_Impulsion", "medida", "float",
        {"db_number": 20, "offset": 68, "bit": 0, "s7_type": "real"},
        description="Caudal impulsado hacia el depósito regulador", unit="m3/h", read_write="read",
        min_value=0, max_value=500,
    )

    # =========================================================================
    # Site 4: Sector de distribución (válvulas reguladoras de presión)
    # =========================================================================
    add_site("centro", "Sector Centro - Red de Distribución")
    add_asset("centro_root", "centro", "Sector Centro", "distribution_sector")
    add_asset("centro_prv1", "centro", "Válvula Reguladora Presión Centro", "prv", "centro_root")
    add_asset("centro_prv2", "centro", "Válvula Reguladora Presión Este", "prv", "centro_root")
    add_asset("centro_arqueta", "centro", "Arqueta Telemedida Plaza Mayor", "metering", "centro_root")
    add_asset("centro_purga", "centro", "Válvula de Purga / Descarga Red", "valve", "centro_root")

    add_connection(
        "conn_centro",
        "centro_root",
        "RTU-Centro-01",
        "opcua",
        {
            "endpoint_url": "opc.tcp://10.20.3.10:4840",
            "security_policy": "http://opcfoundation.org/UA/SecurityPolicy#Basic256Sha256",
            "credentials_ref": "cred-centro-01",
        },
    )

    add_tag(
        "tag_prv1_consigna", "conn_centro", "centro_prv1", "PRV_Centro_Consigna", "consigna", "float",
        {"node_id": "ns=2;s=PRV1.Consigna"}, description="Consigna de presión de salida PRV Centro",
        unit="bar", read_write="read_write", min_value=1.5, max_value=6,
    )
    add_tag(
        "tag_prv1_presion", "conn_centro", "centro_prv1", "PRV_Centro_Presion_Aguas_Abajo", "medida", "float",
        {"node_id": "ns=2;s=PRV1.PresionAguasAbajo"}, description="Presión aguas abajo de la PRV Centro",
        unit="bar", read_write="read", min_value=0, max_value=10,
    )
    add_tag(
        "tag_prv1_apertura", "conn_centro", "centro_prv1", "PRV_Centro_Apertura", "medida", "float",
        {"node_id": "ns=2;s=PRV1.Apertura"}, description="Porcentaje de apertura del obturador",
        unit="%", read_write="read", min_value=0, max_value=100,
    )
    add_tag(
        "tag_prv2_consigna", "conn_centro", "centro_prv2", "PRV_Este_Consigna", "consigna", "float",
        {"node_id": "ns=2;s=PRV2.Consigna"}, description="Consigna de presión de salida PRV Este",
        unit="bar", read_write="read_write", min_value=1.5, max_value=6,
    )
    add_tag(
        "tag_prv2_presion", "conn_centro", "centro_prv2", "PRV_Este_Presion_Aguas_Abajo", "medida", "float",
        {"node_id": "ns=2;s=PRV2.PresionAguasAbajo"}, description="Presión aguas abajo de la PRV Este",
        unit="bar", read_write="read", min_value=0, max_value=10,
    )
    add_tag(
        "tag_caudal_plaza", "conn_centro", "centro_arqueta", "Caudal_Plaza_Mayor", "medida", "float",
        {"node_id": "ns=2;s=Arqueta.Caudal"}, description="Caudal telemedido en Plaza Mayor",
        unit="m3/h", read_write="read", min_value=0, max_value=150,
    )
    add_tag(
        "tag_presion_plaza", "conn_centro", "centro_arqueta", "Presion_Plaza_Mayor", "medida", "float",
        {"node_id": "ns=2;s=Arqueta.Presion"}, description="Presión de red en Plaza Mayor",
        unit="bar", read_write="read", min_value=0, max_value=10,
    )
    add_tag(
        "tag_purga_cmd", "conn_centro", "centro_purga", "ValvPurga_Cmd", "telemando", "bool",
        {"node_id": "ns=2;s=Purga.Cmd"}, description="Orden de apertura de la válvula de purga de red",
        requires_ack=True,
    )
    add_tag(
        "tag_purga_estado", "conn_centro", "centro_purga", "ValvPurga_Estado", "medida", "enum",
        {"node_id": "ns=2;s=Purga.Estado"}, description="Estado de la válvula de purga",
        read_write="read", allowed_values=["abierta", "cerrada"],
    )

    # =========================================================================
    # Schedules (programaciones)
    # =========================================================================
    add_schedule(
        "sch_arranque_b1", "tag_bombeo_b1_marcha", "Arranque bombeo hora punta mañana",
        True, "cron", cron_expr="0 6 * * *",
    )
    add_schedule(
        "sch_parada_b1", "tag_bombeo_b1_marcha", "Parada bombeo valle nocturno",
        False, "cron", cron_expr="0 23 * * *",
    )
    add_schedule(
        "sch_presion_punta", "tag_presion_consigna_colector", "Incremento consigna presión hora punta",
        4.5, "cron", cron_expr="0 7 * * *",
    )
    add_schedule(
        "sch_presion_valle", "tag_presion_consigna_colector", "Reducción consigna presión valle nocturno",
        3.2, "cron", cron_expr="0 23 * * *",
    )
    add_schedule(
        "sch_purga_semanal", "tag_purga_cmd", "Purga semanal red Sector Centro",
        True, "cron", cron_expr="0 5 * * 3", requires_confirmation=True,
    )
    add_schedule(
        "sch_dosificacion_matinal", "tag_dosificador_consigna", "Ajuste consigna cloro turno mañana",
        0.8, "cron", cron_expr="0 6 * * *",
    )
    add_schedule(
        "sch_prueba_reserva", "tag_bombeo_b3_marcha", "Prueba mensual bomba de reserva",
        True, "once", start_date=now_iso(offset_days=5, hour=10),
        requires_confirmation=True,
    )
    add_schedule(
        "sch_cierre_dep2_mantenimiento", "tag_valv_ent2_cmd", "Cierre válvula entrada Depósito 2 (mantenimiento)",
        "cerrar", "once", start_date=now_iso(offset_days=3, hour=8),
        requires_confirmation=True,
    )
    add_schedule(
        "sch_ajuste_nivel_dep1_verano", "tag_nivel_dep1_consigna", "Ajuste consigna nivel Depósito 1 (campaña verano)",
        90, "once", start_date=now_iso(offset_days=1, hour=0),
    )
    add_schedule(
        "sch_apertura_prv2_evento", "tag_prv2_consigna", "Refuerzo presión PRV Este (evento festivo)",
        5.0, "once", start_date=now_iso(offset_days=10, hour=18),
    )

    # =========================================================================
    # Interlocks (enclavamientos)
    # =========================================================================
    add_interlock(
        "tag_bombeo_b1_marcha", "tag_nivel_dep1", "gte", 98,
        action="block",
    )  # no arrancar bombeo si el depósito 1 está prácticamente lleno
    add_interlock(
        "tag_bomba_capt1_marcha", "tag_nivel_pozo", "lt", 15,
        action="block",
    )  # no arrancar captación si el pozo está bajo mínimos
    add_interlock(
        "tag_purga_cmd", "tag_presion_colector", "lt", 2,
        action="warn",
    )  # avisar si se intenta purgar con poca presión en cabecera
    add_interlock(
        "tag_valv_ent1_cmd", "tag_nivel_dep1", "gte", 98,
        action="block",
    )  # no abrir más la entrada si el depósito 1 ya está lleno
    add_interlock(
        "tag_bomba_capt2_marcha", "tag_turbidez", "gt", 5,
        action="warn",
    )  # avisar si se arranca la bomba de reserva con turbidez alta aguas abajo

    # =========================================================================
    # Events (ocurrencias de calendario ligadas a schedules)
    # =========================================================================
    add_event("sch_prueba_reserva", now_iso(offset_days=5, hour=10))
    add_event("sch_cierre_dep2_mantenimiento", now_iso(offset_days=3, hour=8))
    add_event("sch_ajuste_nivel_dep1_verano", now_iso(offset_days=1, hour=0))
    add_event("sch_apertura_prv2_evento", now_iso(offset_days=10, hour=18))
    add_event("sch_purga_semanal", now_iso(offset_days=2, hour=5))
    add_event("sch_arranque_b1", now_iso(offset_days=0, hour=6))
    add_event("sch_parada_b1", now_iso(offset_days=0, hour=23))

    con.commit()

    print("\nDatos de prueba insertados:")
    for t in ("sites", "assets", "connections", "tags", "schedules", "interlocks", "events"):
        n = cur.execute(f"SELECT COUNT(*) FROM {t}").fetchone()[0]
        print(f"  {t:14s} total = {n}")

    con.close()


if __name__ == "__main__":
    main()
