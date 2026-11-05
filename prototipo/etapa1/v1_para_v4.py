# Throwaway (#8): cópia do SQLite da v1 -> arquivos schemaVersion 4 para o protótipo. Só lê a CÓPIA.
import html, json, re, sqlite3, sys, unicodedata
from pathlib import Path

db, saida = Path(sys.argv[1]), Path(sys.argv[2])
saida.mkdir(parents=True, exist_ok=True)
c = sqlite3.connect(f"file:{db.as_posix()}?mode=ro", uri=True)
c.row_factory = sqlite3.Row

def md(h):  # ponytail: só <p>/<br>/<strong>/<em>; a migração real usa htmd (ADR-0014)
    if h is None: return None
    h = re.sub(r"</p>\s*", "\n\n", h); h = re.sub(r"<br\s*/?>", "\n", h)
    h = re.sub(r"</?(strong|b)>", "**", h); h = re.sub(r"</?(em|i)>", "_", h)
    t = html.unescape(re.sub(r"<[^>]+>", "", h)).strip()
    return t or None

def slug(t):
    t = "".join(ch for ch in unicodedata.normalize("NFD", t) if not unicodedata.combining(ch)).lower()
    return re.sub(r"[^a-z0-9]+", "-", t).strip("-") or "registro"

for r in c.execute("select * from registros order by rowid").fetchall():
    ativs = c.execute("select * from atividades where registro_id=? order by rowid", (r["id"],)).fetchall()
    ents = c.execute("select * from entidades where registro_id=? order by rowid", (r["id"],)).fetchall()
    ag_ids = {a["agente_id"] for a in ativs}
    ags = [a for a in c.execute("select * from agentes order by rowid") if a["id"] in ag_ids]
    doc = {
        "schemaVersion": 4,
        "registro": {
            "id": r["id"], "titulo": r["titulo"], "descricao": md(r["descricao"]), "status": r["status"],
            "direcaoDiagrama": r["direcao_diagrama"], "criadoEm": r["criado_em"],
            "atualizadoEm": r["finalizado_em"] or r["criado_em"], "finalizadoEm": r["finalizado_em"],
        },
        "agentes": [{"id": a["id"], "nome": a["nome"], "tipo": a["tipo"], "afiliacao": a["afiliacao"],
                     "identificadorExterno": a["identificador_externo"]} for a in ags],
        "entidades": [{"id": e["id"], "nome": e["nome"], "descricao": e["descricao"], "formato": e["formato"],
                       "localizacao": e["localizacao"], "licenca": e["licenca"],
                       "geradaPorAtividadeId": e["gerada_por_atividade_id"],
                       "tipoRelacaoOrigem": e["tipo_relacao_origem"], "revisaoDeId": e["revisao_de_id"]} for e in ents],
        "atividades": [{"id": a["id"], "tipo": a["tipo"], "agenteId": a["agente_id"], "dataHora": a["data_hora"],
                        "descricao": a["descricao"],
                        "entidadesUsadas": [u[0] for u in c.execute(
                            "select entidade_id from atividade_entidades_usadas where atividade_id=? order by rowid", (a["id"],))],
                        "local": a["local"], "instrumento": a["instrumento"], "processo": a["processo"],
                        "parametros": json.loads(a["parametros"]) if a["parametros"] else None,
                        "ambienteExecucao": json.loads(a["ambiente_execucao"]) if a["ambiente_execucao"] else None}
                       for a in ativs],
    }
    nome = f"{slug(r['titulo'])}-{r['id'].replace('-', '')[-8:]}.json"
    (saida / nome).write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(nome, len(ags), len(ents), len(ativs))
