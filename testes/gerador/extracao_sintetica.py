#!/usr/bin/env python3
"""Gera uma extração SINTÉTICA para testar a importação do SIED.

ATENÇÃO: todos os dados são inventados. A organização das pastas NÃO representa
nenhuma ferramenta de extração (Avilla, IPED...). Apenas o nome `mmssms.db` e o
esquema da tabela `sms` seguem o padrão do Android (AOSP), para exercitar o parser.

Com --whatsapp, acrescenta os bancos do WhatsApp montados a partir das fixtures
sintéticas de testes/fixtures/whatsapp (msgstore.db no esquema novo e wa.db), mídias
citadas pelas mensagens e um backup .crypt14 inventado. Sem a opção, a saída é a
mesma de sempre (é a que o teste E2E usa).

Com --android, acrescenta a agenda (contacts2.db), o registro de chamadas
(calllog.db) e MMS no mmssms.db (tabelas pdu/addr/part, com os arquivos das partes
em app_parts), montados a partir das fixtures sintéticas de testes/fixtures/android
(esquemas públicos do AOSP; números, nomes e arquivos inventados).

Com --volume, acrescenta volume para testar as telas com dados em quantidade: uma
conversa SMS com milhares de mensagens, centenas de conversas curtas e centenas de
imagens PNG desenhadas aqui mesmo (padrões geométricos, sem pessoas nem texto).

Com --iped, a saída é outra: um caso SINTÉTICO do IPED, copiado de testes/fixtures/iped/caso
(pasta iped/ com IPEDConfig.txt, conf/ e index/ sem índice real, "Lista de Arquivos.csv" com as
15 colunas do IPED e dois arquivos em Exportados/arquivos). Não se combina com as outras opções.

Com --adb, a saída é outra: um backup SINTÉTICO do `adb backup` (backup.ab, versão 5,
comprimido com zlib, sem cifra) com os bancos das fixtures do Android e do WhatsApp em
apps/<pacote>/db/ e um _manifest por app, no formato público do Android. Não se combina com as
outras opções.

Uso: python3 testes/gerador/extracao_sintetica.py [--whatsapp] [--android] [--volume] <pasta de destino>
     python3 testes/gerador/extracao_sintetica.py --iped <pasta de destino>
     python3 testes/gerador/extracao_sintetica.py --adb <pasta de destino>
"""

import os
import shutil
import io
import sqlite3
import struct
import sys
import tarfile
import tempfile
import zlib

FIXTURES_WHATSAPP = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "fixtures", "whatsapp")
FIXTURES_ANDROID = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "fixtures", "android")
FIXTURE_IPED = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "fixtures", "iped", "caso")

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


# Volume (--volume): ids a partir de 1000 para não colidir com as mensagens acima.
VOLUME_CONVERSA_LONGA = 5000
VOLUME_CONVERSAS_CURTAS = 300
VOLUME_IMAGENS = 400


def mensagens_volume():
    """Mensagens sintéticas em quantidade; o texto diz que são inventadas."""
    i = 1000
    for n in range(VOLUME_CONVERSA_LONGA):
        marcador = " [marcador sintético]" if n % 250 == 0 else ""
        yield (i, 10, "+5561900000010", 200 + n * 7, 1 if n % 3 else 2,
               f"Mensagem sintética {n + 1} da conversa longa{marcador}")
        i += 1
    for c in range(VOLUME_CONVERSAS_CURTAS):
        numero = f"+55619{(c + 100):08d}"
        for k in range(3):
            yield (i, 100 + c, numero, 300 + c * 40 + k, 1 if k != 1 else 2,
                   f"Conversa sintética {c + 1}, mensagem {k + 1}")
            i += 1


def png(largura: int, altura: int, semente: int) -> bytes:
    """PNG RGB com um padrão de faixas e círculo, sem bibliotecas externas."""
    r0, g0, b0 = (semente * 53) % 256, (semente * 97) % 256, (semente * 151) % 256
    cx, cy, raio2 = largura // 2, altura // 2, (min(largura, altura) // 3) ** 2
    linhas = bytearray()
    for y in range(altura):
        linhas.append(0)
        for x in range(largura):
            if (x - cx) ** 2 + (y - cy) ** 2 < raio2:
                linhas += bytes((255 - r0, 255 - g0, 255 - b0))
            elif ((x + semente * 5) // 24 + y // 24) % 2:
                linhas += bytes((r0, g0, b0))
            else:
                linhas += bytes((r0 // 2, g0 // 2, b0 // 2))

    def bloco(tipo: bytes, dados: bytes) -> bytes:
        return struct.pack(">I", len(dados)) + tipo + dados + struct.pack(">I", zlib.crc32(tipo + dados) & 0xFFFFFFFF)

    cabecalho = struct.pack(">IIBBBBB", largura, altura, 8, 2, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + bloco(b"IHDR", cabecalho) + bloco(b"IDAT", zlib.compress(bytes(linhas), 9)) + bloco(b"IEND", b"")


def gerar_imagens(destino: str) -> None:
    pasta = os.path.join(destino, "midia", "DCIM", "Volume")
    os.makedirs(pasta)
    for n in range(VOLUME_IMAGENS):
        largura, altura = (640, 480) if n % 4 else (360, 640)
        with open(os.path.join(pasta, f"IMG_SINT_{n + 1:04d}.png"), "wb") as f:
            f.write(png(largura, altura, n + 1))


def banco_de_fixture(sql: str, destino: str, pasta: str = FIXTURES_WHATSAPP) -> None:
    with open(os.path.join(pasta, sql), encoding="utf-8") as f:
        script = f.read()
    conn = sqlite3.connect(destino)
    conn.executescript(script)
    conn.close()


def arquivo(caminho: str, inicio: bytes, tamanho: int) -> None:
    os.makedirs(os.path.dirname(caminho), exist_ok=True)
    with open(caminho, "wb") as f:
        f.write(inicio + b"\x00" * (tamanho - len(inicio)))


def gerar_whatsapp(destino: str) -> None:
    """Pastas inventadas; só os nomes dos bancos e das pastas de mídia seguem o app."""
    bancos = os.path.join(destino, "dados", "com.whatsapp", "databases")
    os.makedirs(bancos)
    banco_de_fixture("msgstore_novo.sql", os.path.join(bancos, "msgstore.db"))
    banco_de_fixture("wa.sql", os.path.join(bancos, "wa.db"))
    midia = os.path.join(destino, "armazenamento", "WhatsApp", "Media")
    # A foto e o documento citados pelas mensagens existem; o áudio não.
    arquivo(os.path.join(midia, "WhatsApp Images", "IMG-20260102-WA0001.jpg"), b"\xff\xd8\xff\xe0", 64)
    arquivo(os.path.join(midia, "WhatsApp Documents", "relatorio-sintetico.pdf"), b"%PDF-1.4\n% SINTETICO\n", 128)
    # Backup cifrado: só inventariado, nunca interpretado.
    arquivo(os.path.join(destino, "armazenamento", "WhatsApp", "Databases", "msgstore-2026-01-02.1.db.crypt14"),
            b"SINTETICO-CIFRADO", 256)


def gerar_android(destino: str) -> None:
    """Pasta inventada; só o nome do pacote e dos bancos seguem o Android."""
    bancos = os.path.join(destino, "dados", "com.android.providers.contacts", "databases")
    os.makedirs(bancos)
    banco_de_fixture("contacts2.sql", os.path.join(bancos, "contacts2.db"), FIXTURES_ANDROID)
    banco_de_fixture("calllog.sql", os.path.join(bancos, "calllog.db"), FIXTURES_ANDROID)


def gerar(destino: str, whatsapp: bool = False, volume: bool = False, android: bool = False) -> None:
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
        if android:
            with open(os.path.join(FIXTURES_ANDROID, "mms.sql"), encoding="utf-8") as f:
                conn.executescript(f.read())
        if volume:
            conn.execute("BEGIN")
            for (i, t, a, m, tipo, corpo) in mensagens_volume():
                conn.execute(inserir, (i, t, a, BASE_MS + m * MINUTO, tipo, corpo))
            conn.execute("COMMIT")
        conn.execute("PRAGMA wal_checkpoint(TRUNCATE)")
        conn.execute("PRAGMA wal_autocheckpoint=0")
        for (i, t, a, m, tipo, corpo) in MENSAGENS_WAL:
            conn.execute(inserir, (i, t, a, BASE_MS + m * MINUTO, tipo, corpo))
        pasta_db = os.path.join(destino, "dados", "com.android.providers.telephony", "databases")
        # Copia com a conexão aberta: o -wal ainda contém a última mensagem.
        shutil.copyfile(db, os.path.join(pasta_db, "mmssms.db"))
        shutil.copyfile(db + "-wal", os.path.join(pasta_db, "mmssms.db-wal"))
        conn.close()
    if whatsapp:
        gerar_whatsapp(destino)
    if android:
        gerar_android(destino)
        partes = os.path.join(destino, "dados", "com.android.providers.telephony", "app_parts")
        # Foto e áudio citados pelos MMS; a segunda imagem do MMS 3 fica de fora.
        os.makedirs(partes, exist_ok=True)
        with open(os.path.join(partes, "PART_1767232800000_IMG_0002.png"), "wb") as f:
            f.write(png(96, 72, 7))
        arquivo(os.path.join(partes, "PART_1767240000000_audio_sintetico.amr"), b"#!AMR\n", 32)
    if volume:
        gerar_imagens(destino)
    print(destino)


def gerar_iped(destino: str) -> None:
    if os.path.exists(destino) and os.listdir(destino):
        sys.exit(f"a pasta {destino} já existe e não está vazia")
    if os.path.exists(destino):
        os.rmdir(destino)
    shutil.copytree(FIXTURE_IPED, destino)
    print(destino)


def gerar_adb(destino: str) -> None:
    """Pastas do tar seguem o formato público do `adb backup` (apps/<pacote>/db, _manifest)."""
    if os.path.exists(destino) and os.listdir(destino):
        sys.exit(f"a pasta {destino} já existe e não está vazia")
    os.makedirs(destino, exist_ok=True)
    sms = ("CREATE TABLE sms (_id INTEGER PRIMARY KEY, thread_id INTEGER, address TEXT, date INTEGER, "
           "type INTEGER, body TEXT, read INTEGER DEFAULT 1, seen INTEGER DEFAULT 1);")
    with tempfile.TemporaryDirectory() as tmp:
        def banco(nome: str, scripts: list) -> bytes:
            p = os.path.join(tmp, nome)
            conn = sqlite3.connect(p)
            for s in scripts:
                conn.executescript(s)
            conn.close()
            with open(p, "rb") as f:
                return f.read()

        def fixture(pasta: str, nome: str) -> str:
            with open(os.path.join(pasta, nome), encoding="utf-8") as f:
                return f.read()

        inserir = "".join(
            f"INSERT INTO sms(_id, thread_id, address, date, type, body) VALUES "
            f"({i}, {t}, '{a}', {BASE_MS + m * MINUTO}, {tipo}, '{corpo}');"
            for (i, t, a, m, tipo, corpo) in MENSAGENS
        )
        tel, cont, wa = ("apps/com.android.providers.telephony", "apps/com.android.providers.contacts",
                         "apps/com.whatsapp")
        manifesto = lambda p: f"1\n{p}\n34\n34\n\n0\n0\n".encode()
        itens = [
            (f"{tel}/_manifest", manifesto("com.android.providers.telephony")),
            (f"{tel}/db/mmssms.db", banco("mmssms.db", [sms, inserir, fixture(FIXTURES_ANDROID, "mms.sql")])),
            (f"{cont}/_manifest", manifesto("com.android.providers.contacts")),
            (f"{cont}/db/contacts2.db", banco("contacts2.db", [fixture(FIXTURES_ANDROID, "contacts2.sql")])),
            (f"{cont}/db/calllog.db", banco("calllog.db", [fixture(FIXTURES_ANDROID, "calllog.sql")])),
            (f"{wa}/_manifest", manifesto("com.whatsapp")),
            (f"{wa}/db/msgstore.db", banco("msgstore.db", [fixture(FIXTURES_WHATSAPP, "msgstore_novo.sql")])),
            (f"{wa}/db/wa.db", banco("wa.db", [fixture(FIXTURES_WHATSAPP, "wa.sql")])),
        ]
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w", format=tarfile.USTAR_FORMAT) as t:
        for nome, dados in itens:
            info = tarfile.TarInfo(nome)
            info.size = len(dados)
            info.mtime = BASE_MS // 1000
            info.mode = 0o600
            t.addfile(info, io.BytesIO(dados))
    with open(os.path.join(destino, "backup.ab"), "wb") as f:
        f.write(b"ANDROID BACKUP\n5\n1\nnone\n")
        f.write(zlib.compress(buf.getvalue()))
    with open(os.path.join(destino, "LEIAME-SINTETICO.txt"), "w", encoding="utf-8") as f:
        f.write("Backup SINTÉTICO (adb backup) gerado para testes do SIED. Nenhum dado é real.\n")
    print(destino)


if __name__ == "__main__":
    args = sys.argv[1:]
    if "--adb" in args:
        resto = [a for a in args if a != "--adb"]
        if len(resto) != 1 or resto[0].startswith("--"):
            sys.exit(__doc__)
        gerar_adb(resto[0])
        sys.exit(0)
    if "--iped" in args:
        resto = [a for a in args if a != "--iped"]
        if len(resto) != 1 or resto[0].startswith("--"):
            sys.exit(__doc__)
        gerar_iped(resto[0])
        sys.exit(0)
    whatsapp = "--whatsapp" in args
    volume = "--volume" in args
    android = "--android" in args
    args = [a for a in args if a not in ("--whatsapp", "--volume", "--android")]
    if len(args) != 1:
        sys.exit(__doc__)
    gerar(args[0], whatsapp, volume, android)
