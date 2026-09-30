//! Lista dos importadores e parsers disponíveis nesta versão do SIED.

use crate::fonte::FonteSomenteLeitura;
use crate::importador::{Deteccao, Importador, Suporte};
use crate::importadores::avilla::AvillaImporter;
use crate::importadores::generico::ImportadorGenerico;
use crate::importadores::iped::IpedImporter;
use crate::parser::Parser;
use crate::parsers::android_chamadas::ParserChamadasAndroid;
use crate::parsers::android_contatos::ParserContatosAndroid;
use crate::parsers::android_mms::ParserMmsAndroid;
use crate::parsers::android_sms::ParserSmsAndroid;
use crate::parsers::iped_lista::ParserListaIped;
use crate::parsers::whatsapp_android::ParserWhatsAppAndroid;
use crate::Resultado;

#[derive(Default)]
pub struct Registro {
    importadores: Vec<Box<dyn Importador>>,
    parsers: Vec<Box<dyn Parser>>,
}

impl Registro {
    /// Componentes embutidos nesta versão.
    pub fn padrao() -> Self {
        let mut r = Registro::default();
        r.adicionar_importador(Box::new(AvillaImporter));
        r.adicionar_importador(Box::new(IpedImporter));
        r.adicionar_importador(Box::new(ImportadorGenerico));
        r.adicionar_parser(Box::new(ParserSmsAndroid));
        r.adicionar_parser(Box::new(ParserWhatsAppAndroid));
        r.adicionar_parser(Box::new(ParserContatosAndroid));
        r.adicionar_parser(Box::new(ParserChamadasAndroid));
        r.adicionar_parser(Box::new(ParserMmsAndroid));
        r.adicionar_parser(Box::new(ParserListaIped));
        r
    }

    pub fn adicionar_importador(&mut self, i: Box<dyn Importador>) {
        self.importadores.push(i);
    }

    pub fn adicionar_parser(&mut self, p: Box<dyn Parser>) {
        self.parsers.push(p);
    }

    pub fn importadores(&self) -> impl Iterator<Item = &dyn Importador> {
        self.importadores.iter().map(|b| b.as_ref())
    }

    pub fn importador(&self, id: &str) -> Option<&dyn Importador> {
        self.importadores().find(|i| i.info().id == id)
    }

    pub fn parser(&self, id: &str) -> Option<&dyn Parser> {
        self.parsers
            .iter()
            .map(|b| b.as_ref())
            .find(|p| p.info().id == id)
    }

    /// Pergunta a todos os importadores e ordena do mais provável ao menos provável.
    /// A escolha final é sempre confirmada pelo analista.
    pub fn detectar(&self, fonte: &dyn FonteSomenteLeitura) -> Resultado<Vec<Deteccao>> {
        let mut todas = Vec::new();
        for i in self.importadores() {
            todas.push(i.detectar(fonte)?);
        }
        let peso = |s: Suporte| match s {
            Suporte::Completo => 3,
            Suporte::Parcial => 2,
            Suporte::NaoImplementado => 1,
            Suporte::NaoReconhecido => 0,
        };
        todas.sort_by(|a, b| {
            peso(b.suporte)
                .cmp(&peso(a.suporte))
                .then(b.confianca.total_cmp(&a.confianca))
        });
        Ok(todas)
    }
}
