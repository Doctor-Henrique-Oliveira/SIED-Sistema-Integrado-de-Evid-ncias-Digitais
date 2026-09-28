#!/usr/bin/env python3
"""Gera uma extração SINTÉTICA para testar a importação do SIED.

ATENÇÃO: todos os dados são inventados. A organização das pastas NÃO representa
nenhuma ferramenta de extração (Avilla, IPED...). Apenas o nome `mmssms.db` e o
esquema da tabela `sms` seguem o padrão do Android (AOSP), para exercitar o parser.

Uso: python3 testes/gerador/extracao_sintetica.py <pasta de destino>
"""

import os
import shutil
import sqlite3
import sys
import tempfile

# 2026-01-01 00:00:00 UTC em milissegundos
BASE_MS = 1767225600000
MINUTO = 60_000

MENSAGENS = [
    # (_id, thread_id, address, minutos após BASE, type, body)
    (1, 1, "+5511900000001", 0, 1, "Oi, tudo bem? (mensagem sintética)"),
    (2, 1, "+5511900000001", 2, 2, "Tudo sim, e você?"),
    (3, 1, "+5511900000001", 5, 1, "Vamos nos encontrar amanhã às 10h?"),
    (4, 2, "+5521900000002", 60, 1, "Seu código de verificação é 000000 (sintético)"),
    (5, 1, "+5511900000001", 90, 2, "Combinado."),
]
# Gravadas só no -wal (banco não consolidado), como ocorre em aparelhos reais.
MENSAGENS_WAL = [
    (6, 3, "+5531900000003", 120, 1, "Mensagem que só existe no WAL (sintético)"),
]


def gerar(destino: str) -> None:
    if os.path.exists(destino) and os.listdir(destino):
        sys.exit(f"a pasta {destino} já existe e não está vazia")
    os.makedirs(os.path.join(destino, "dados", "com.android.providers.telephony", "databases"))
    os.makedirs(os.path.join(destino, "midia", "DCIM"))
    with open(os.path.join(destino, "LEIAME-SINTETICO.txt"), "w", encoding="utf-8") as f:
        f.write("Extração SINTÉTICA gerada para testes do SIED. Nenhum dado é real.\n")
    # Assinatura de JPEG com extensão trocada, para testar a classificação pelo conteúdo.
    with open(os.path.join(destino, "midia", "DCIM", "IMG_0001.dat"), "wb") as f:
        f.write(b"\xff\xd8\xff\xe0" + b"\x00" * 60)
    with open(os.path.join(destino, "midia", "vazio.txt"), "wb"):
        pass

    with tempfile.TemporaryDirectory() as tmp:
        db = os.path.join(tmp, "mmssms.db")
        conn = sqlite3.connect(db, isolation_level=None)
        conn.executescript(
            """
            PRAGMA journal_mode=WAL;
            CREATE TABLE sms (_id INTEGER PRIMARY KEY, thread_id INTEGER, address TEXT, person INTEGER,
                date INTEGER, date_sent INTEGER DEFAULT 0, protocol INTEGER, read INTEGER DEFAULT 0,
                status INTEGER DEFAULT -1, type INTEGER, reply_path_present INTEGER, subject TEXT,
                body TEXT, service_center TEXT, locked INTEGER DEFAULT 0, sub_id INTEGER DEFAULT -1,
                error_code INTEGER DEFAULT 0, creator TEXT, seen INTEGER DEFAULT 0);
            """
        )
        inserir = ("INSERT INTO sms(_id, thread_id, address, date, type, body, read, seen) "
                   "VALUES (?, ?, ?, ?, ?, ?, 1, 1)")
        for (i, t, a, m, tipo, corpo) in MENSAGENS:
            conn.execute(inserir, (i, t, a, BASE_MS + m * MINUTO, tipo, corpo))
        conn.execute("PRAGMA wal_checkpoint(TRUNCATE)")
        conn.execute("PRAGMA wal_autocheckpoint=0")
        for (i, t, a, m, tipo, corpo) in MENSAGENS_WAL:
            conn.execute(inserir, (i, t, a, BASE_MS + m * MINUTO, tipo, corpo))
        pasta_db = os.path.join(destino, "dados", "com.android.providers.telephony", "databases")
        # Copia com a conexão aberta: o -wal ainda contém a última mensagem.
        shutil.copyfile(db, os.path.join(pasta_db, "mmssms.db"))
        shutil.copyfile(db + "-wal", os.path.join(pasta_db, "mmssms.db-wal"))
        conn.close()
    print(destino)


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    gerar(sys.argv[1])
