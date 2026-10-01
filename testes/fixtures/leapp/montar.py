"""Monta as fixtures SINTÉTICAS do ALEAPP e do iLEAPP (nada aqui veio de um relatório real).

Os TSV são gravados como o próprio xLEAPP grava (função `tsv` de scripts/ilapfuncs.py nos
projetos públicos abrignoni/ALEAPP e abrignoni/iLEAPP): csv.writer do Python com separador
TAB, UTF-8 com BOM, arquivo aberto em acréscimo (o cabeçalho se repete a cada gravação).
Nomes dos artefatos e das colunas seguem os módulos públicos; valores são inventados.

Uso: python3 testes/fixtures/leapp/montar.py   (regrava aleapp/ e ileapp/ ao lado deste arquivo)
"""

import codecs
import csv
import json
import os
import shutil

AQUI = os.path.dirname(os.path.abspath(__file__))


def tsv(pasta, nome, cabecalho, linhas):
    destino = os.path.join(pasta, "_TSV Exports")
    os.makedirs(destino, exist_ok=True)
    with codecs.open(os.path.join(destino, nome + ".tsv"), "a", "utf-8-sig") as f:
        w = csv.writer(f, delimiter="\t")
        w.writerow(cabecalho)
        for l in linhas:
            w.writerow(l)


def projeto(pasta, nome, versao, entrada, tipo):
    dados = {
        "lava_schema_version": "SINTETICO",
        "parser_info": {"leapp_name": nome, "leapp_version": versao, "leapp_mode": "CLI",
                        "package": "Source code", "OS": "SINTETICO", "start_timestamp": 1767358800},
        "param_input": entrada,
        "param_output": "/sintetico/saida",
        "param_type": tipo,
        "param_profile": None,
        "profile_used": False,
        "processing_status": "Complete",
        "lava_db_name": "_lava_artifacts.db",
        "modules": [],
        "artifacts": {},
        "meta": {"modules": []},
    }
    with open(os.path.join(pasta, "_lava_data.lava"), "w", encoding="utf-8") as f:
        json.dump(dados, f, indent=4)
    os.makedirs(os.path.join(pasta, "_HTML"), exist_ok=True)
    with open(os.path.join(pasta, "_HTML", "index.html"), "w", encoding="utf-8") as f:
        f.write("<!-- relatório HTML SINTÉTICO: não é lido pelo SIED -->\n")


def aleapp():
    p = os.path.join(AQUI, "aleapp")
    shutil.rmtree(p, ignore_errors=True)
    os.makedirs(p)
    projeto(p, "ALEAPP", "2026.4.2", "/sintetico/extracao-SINTETICA.tar", "tar")
    tsv(p, "SMS Messages",
        ["Date", "Date Sent", "Type", "Address", "Body", "MSG ID", "Thread ID", "Contact ID",
         "Read", "Service Center", "Error Code"],
        [["2026-01-02 13:00:00+00:00", "2026-01-02 12:59:58+00:00", "Received", "+5561900000001",
          "Mensagem sintética 1", "1", "1", "", "1", "", "0"],
         ["2026-01-02 13:05:00.250000+00:00", "", "Sent", "+5561900000001",
          "Com\ttab, \"aspas\" e\nduas linhas", "2", "1", "", "1", "", "0"],
         ["2026-01-03 09:30:00-03:00", "", "Received", "+5561900000002", "Mensagem sintética 3",
          "3", "2", "", "0", "", "0"]])
    # Nome real do módulo tem espaço no fim ("Call logs "); duas gravações = cabeçalho repetido.
    cab = ["Call Date", "Phone Account Address", "Partner", "Type", "Duration in Secs",
           "Partner Location", "Country ISO", "Data", "Mime Type", "Transcription", "Deleted"]
    tsv(p, "Call logs ", cab,
        [["2026-01-04 18:00:00+00:00", "", "+5561900000003", "Incoming", "42", "", "BR", "", "",
          "", "0"]])
    tsv(p, "Call logs ", cab,
        [["2026-01-04 19:00:00", "", "+5561900000004", "Missed", "0", "", "BR", "", "", "", "1"]])
    tsv(p, "Cache Location",
        ["Accuracy", "Confidence", "Latitude", "Longitude", "Readtime"],
        [["20", "75", "-15.7939", "-47.8828", "2026-01-05 10:00:00+00:00"],
         ["35", "68", "0", "0", ""]])


def ileapp():
    p = os.path.join(AQUI, "ileapp")
    shutil.rmtree(p, ignore_errors=True)
    os.makedirs(p)
    projeto(p, "iLEAPP", "2026.3.1", "/sintetico/backup-itunes-SINTETICO", "itunes")
    # Só as 8 primeiras colunas do módulo "SMS" do iLEAPP.
    tsv(p, "SMS",
        ["Message Timestamp", "Read Timestamp", "Delivered Timestamp", "Attachment Timestamp",
         "From Me", "Sender", "Chat Contact ID", "Message"],
        [["2026-01-06 14:00:00+00:00", "", "", "", "0", "+5561900000005", "+5561900000005",
          "iMessage sintética"]])


if __name__ == "__main__":
    aleapp()
    ileapp()
