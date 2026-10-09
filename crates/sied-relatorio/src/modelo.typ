// Modelo do relatório do SIED. Os dados vêm de /dados.json (montados em Rust) e as
// imagens de /img/. Todo texto vindo dos dados é exibido como texto, nunca avaliado.

#let d = json("/dados.json")
#let s = d.secoes

#let tinta = rgb("#1d2733")
#let suave = rgb("#5b6675")
#let linha = rgb("#c9ced6")
#let ouro = rgb("#a57b2c")
#let fundo = rgb("#f4f5f7")
#let ok = rgb("#1f7a55")
#let alerta = rgb("#b3261e")

#set document(title: d.titulo, author: d.analista)
#set text(font: "Libertinus Serif", size: 10.5pt, lang: "pt", region: "br", fill: tinta)
#set par(justify: true, leading: 0.62em)
#set heading(numbering: "1.1")
#show heading.where(level: 1): it => {
  v(0.6em)
  block(below: 0.8em, {
    set text(15pt, weight: "bold")
    it
    v(-0.45em)
    line(length: 100%, stroke: 1.2pt + ouro)
  })
}
#show heading.where(level: 2): it => block(above: 1.1em, below: 0.6em, text(12pt, weight: "bold", it))

#let mono(t, size: 7.6pt) = text(font: "DejaVu Sans Mono", size: size, t)
#let hexa = regex("^[0-9a-f]{64}$")
// Hashes em fonte monoespaçada; o resto como texto.
#let valor(v) = if v.match(hexa) != none { mono(v, size: 8pt) } else { text(size: 9.5pt, v) }
#let rotulo(t) = text(size: 8.5pt, fill: suave, t)

#set page(
  paper: "a4",
  margin: (x: 2.1cm, top: 2.3cm, bottom: 2.2cm),
  header: context {
    if counter(page).get().first() > 1 {
      set text(8pt, fill: suave)
      grid(columns: (1fr, auto), d.identificacao, [Relatório SIED])
      v(-0.5em)
      line(length: 100%, stroke: 0.4pt + linha)
    }
  },
  footer: context {
    set text(8pt, fill: suave)
    line(length: 100%, stroke: 0.4pt + linha)
    v(-0.4em)
    grid(
      columns: (1fr, auto),
      [Gerado pelo SIED #d.sied_versao em #d.gerado_em (#d.fuso) por #d.analista],
      [Página #counter(page).display() de #counter(page).final().first()],
    )
  },
)

// Tabela de rótulo e valor.
#let campos(lista, largura: 32%) = {
  if lista.len() > 0 {
    table(
      columns: (largura, 1fr),
      stroke: (x, y) => (bottom: 0.4pt + linha),
      inset: (x: 4pt, y: 4.5pt),
      ..lista.map(c => (rotulo(c.rotulo), valor(c.valor))).flatten(),
    )
  }
}

#let caixa(cor, corpo) = block(
  width: 100%, inset: 8pt, radius: 3pt, fill: cor.lighten(90%), stroke: 0.6pt + cor, corpo,
)

// ------------------------------------------------------------------ capa

#if s.capa {
  page(header: none, {
    v(3.2cm)
    text(9pt, fill: ouro, tracking: 1.5pt, weight: "bold")[SIED · SISTEMA INTEGRADO DE EVIDÊNCIAS DIGITAIS]
    v(0.4cm)
    line(length: 30%, stroke: 2pt + ouro)
    v(0.6cm)
    text(24pt, weight: "bold", d.titulo)
    v(0.3cm)
    text(14pt, fill: suave, d.identificacao)
    v(1.6cm)
    campos(d.capa, largura: 36%)
    v(1fr)
    caixa(ouro, [
      #set text(9pt)
      Documento gerado localmente, sem conexão com a internet. Cada evidência traz o hash registrado quando foi
      selecionada e o hash conferido na geração deste relatório; o apêndice técnico aponta o arquivo, a tabela e o
      registro de origem. A seção de integridade identifica a trilha de auditoria do caso até este ponto.
    ])
    v(0.4cm)
    rotulo[Gerado em #d.gerado_em (#d.fuso) · analista #d.analista · estação #d.estacao]
  })
}

#outline(title: [Sumário], depth: 2, indent: auto)
#pagebreak()

// ------------------------------------------------------------------ seções

#if s.procedimento [
  = Identificação do procedimento
  #campos(d.procedimento)
]

#if s.dispositivos [
  = Dispositivos
  #if d.dispositivos.len() == 0 [Nenhum dispositivo cadastrado no caso.]
  #for x in d.dispositivos [
    == #x.nome
    #campos(x.campos)
  ]
]

#if s.aquisicao [
  = Extrações importadas e hashes
  Cada extração foi importada em modo somente leitura. O SHA-256 de todos os arquivos consta no manifesto da
  importação, cujo próprio hash aparece abaixo.
  #if d.extracoes.len() == 0 [Nenhuma extração importada no caso.]
  #for x in d.extracoes [
    == #x.nome
    #campos(x.campos)
  ]
]

#if s.metodologia [
  = Metodologia
  #for p in d.metodologia [
    #p
    #parbreak()
  ]
  #if d.transcricoes.len() > 0 [
    == Transcrições locais
    Os trechos abaixo são material derivado de execução local. A origem, o hash e o motor permanecem visíveis para revisão; os timestamps permitem localizar o trecho no arquivo original. A transcrição não constitui conclusão pericial.
    #for t in d.transcricoes [
      === #t.origem
      #text(8.5pt, fill: suave)[Estado: #t.estado · Motor: #t.motor]
      #linebreak()
      #text(8.5pt, fill: suave)[SHA-256 da origem: #t.hash_origem]
      #if t.segmentos.len() == 0 [
        #parbreak()
        Nenhum segmento persistido.
      ]
      #for s in t.segmentos [
        #parbreak()
        #text(8.5pt, fill: suave)[#s.inicio → #s.fim#if s.falante != none [ · #s.falante]]
        #linebreak()
        #s.texto
      ]
    ]
  ]
]

// ------------------------------------------------------------------ evidências

#let separador_dia(t) = align(center, block(
  above: 0.7em, below: 0.5em, inset: (x: 8pt, y: 3pt), radius: 8pt, fill: fundo,
  text(8pt, fill: suave, t),
))

#let figura(f) = figure(
  image("/img/" + f.arquivo, width: 72%),
  caption: text(8.5pt, f.legenda),
  supplement: [Figura],
)

#let bolha(m) = {
  if m.sistema {
    align(center, block(width: 80%, inset: 5pt, radius: 4pt, fill: fundo,
      text(8.5pt, fill: suave)[#m.texto #h(0.6em) #m.hora · reg. #m.id]))
  } else {
    let cor = if m.de_mim { rgb("#f6eedc") } else { rgb("#eef1f5") }
    let borda = if m.de_mim { rgb("#dcc79b") } else { linha }
    align(if m.de_mim { right } else { left }, block(
      width: 80%, breakable: true, above: 0.45em, below: 0.45em,
      inset: (x: 8pt, y: 6pt), radius: 5pt, fill: cor, stroke: 0.5pt + borda,
      {
        set par(justify: false)
        grid(columns: (1fr, auto), column-gutter: 6pt,
          text(8.5pt, weight: "bold", m.autor),
          text(7.5pt, fill: suave)[#m.hora · reg. #m.id])
        v(-0.2em)
        if m.texto != none { text(10pt, m.texto) }
        for x in m.detalhes { linebreak(); text(8.5pt, fill: suave, x) }
        if m.figura != none { figura(m.figura) }
      },
    ))
  }
}

#let conversa(c) = {
  campos(c.cabecalho, largura: 26%)
  v(0.4em)
  let dia = none
  for m in c.mensagens {
    if m.dia != dia { separador_dia(m.dia); dia = m.dia }
    bolha(m)
  }
}

#let situacao(e) = {
  let cor = if e.situacao == "conferida" { ok } else { alerta }
  caixa(cor, text(9pt, fill: cor.darken(20%), e.situacao_texto))
}

#if s.evidencias [
  = Evidências selecionadas
  #if d.evidencias.len() == 0 [Nenhuma evidência foi selecionada no caso.]
  #for e in d.evidencias [
    #block(breakable: true, above: 1.4em)[
      == #e.referencia · #e.titulo
      #text(9pt, fill: suave)[#e.tipo#if e.categoria != none [ · #e.categoria]]
      #if e.descricao != none [
        #v(0.2em)
        #e.descricao
      ]
      #v(0.3em)
      #let c = e.conteudo
      #if c.tipo == "conversa" { conversa(c) }
      #if c.tipo == "registro" { campos(c.campos) }
      #if c.tipo == "arquivo" {
        campos(c.campos)
        if c.figura != none { figura(c.figura) }
        if c.sem_figura != none { text(9pt, fill: suave, c.sem_figura) }
      }
      #if e.observacao != none [
        #v(0.3em)
        #caixa(ouro)[*Observação do analista:* #e.observacao]
      ]
      #v(0.3em)
      #situacao(e)
      #v(0.2em)
      #campos(e.custodia, largura: 30%)
    ]
  ]
]

#if s.observacoes and d.observacoes != none [
  = Observações
  #d.observacoes
]

#if s.conclusao and d.conclusao != none [
  = Conclusão do analista
  #d.conclusao
]

// ------------------------------------------------------------------ apêndice

#if s.apendice_tecnico and d.evidencias.len() > 0 [
  = Apêndice técnico: origem de cada evidência
  Para cada evidência: de qual extração, arquivo, tabela e registro o dado foi lido, com o parser e a versão
  que o interpretaram e os valores brutos da linha de origem. Com essas informações, o registro pode ser
  localizado no arquivo original com qualquer ferramenta SQLite.
  #for e in d.evidencias [
    == #e.referencia · #e.titulo
    #for o in e.origem [
      #block(breakable: true)[
        #block(sticky: true, below: 0.5em, text(9.5pt, weight: "bold", o.titulo))
        #campos(o.campos, largura: 28%)
        #for r in o.referencias [
          #campos(r.campos, largura: 28%)
          #if r.valores_brutos != "" [
            #block(width: 100%, inset: 6pt, fill: fundo, radius: 3pt, mono(r.valores_brutos))
          ]
        ]
        #if o.linhas.len() > 0 {
          table(
            columns: (auto, auto, 1fr, auto),
            stroke: (x, y) => (bottom: 0.3pt + linha),
            inset: 3pt,
            ..([Registro], [Data], [Tabela e registro], [hash_registro]).map(t => rotulo(t)),
            ..o.linhas.map(l => (mono(l.at(0)), mono(l.at(1)), text(8pt, l.at(2)), mono(l.at(3), size: 6.6pt))).flatten(),
          )
        }
      ]
    ]
  ]
]

#if s.integridade [
  = Integridade e cadeia de custódia
  #campos(d.integridade.campos, largura: 34%)
  #if d.integridade.evidencias.len() > 0 {
    v(0.4em)
    table(
      columns: (auto, 1fr, auto),
      stroke: (x, y) => (bottom: 0.3pt + linha),
      inset: 4pt,
      rotulo[Evidência], rotulo[Hash registrado na seleção], rotulo[Conferência],
      ..d.integridade.evidencias.map(l => (
        text(9pt, l.at(0)),
        mono(l.at(1), size: 7.4pt),
        text(9pt, weight: "bold", fill: if l.at(2) == "Confere" { ok } else { alerta }, l.at(2)),
      )).flatten(),
    )
  }
]
