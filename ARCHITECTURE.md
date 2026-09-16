# Local Lead Prospector — Arquitetura e Especificação Técnica

## 1. Visão do produto

O **Local Lead Prospector** é um aplicativo desktop para uso pessoal, construído com **Tauri v2 + React/TypeScript + Rust + SQLite**, cujo objetivo é:

1. Descobrir empresas locais a partir de:
   - nicho/categoria;
   - cidade/localidade;
   - raio;
   - região selecionada no mapa;
   - palavras-chave complementares.
2. Maximizar a cobertura de negócios encontrados sem depender de uma única consulta ampla.
3. Deduplicar empresas encontradas em múltiplas fontes e múltiplas células geográficas.
4. Enriquecer leads com dados comerciais públicos:
   - website;
   - telefone;
   - e-mail comercial;
   - Instagram;
   - Facebook;
   - WhatsApp/link público;
   - rating;
   - quantidade de reviews;
   - categoria;
   - endereço;
   - localização.
5. Permitir filtros como:
   - sem website;
   - com website;
   - com Instagram;
   - com e-mail;
   - com WhatsApp/link público;
   - com telefone;
   - rating mínimo;
   - número mínimo de reviews;
   - categoria;
   - origem;
   - status de prospecção.
6. Qualificar leads e gerar mensagens personalizadas.
7. Organizar uma fila de prospecção com revisão humana antes de contato.
8. Acompanhar o estado do lead como um mini CRM.

O produto não deve ser tratado apenas como um scraper. O núcleo do sistema é um **motor de descoberta geográfica + enriquecimento + qualificação + organização de outreach**.

---

# 2. Princípios de arquitetura

## 2.1 Local-first

O aplicativo deve funcionar localmente.

Stack principal:

- Tauri v2
- Rust
- React
- TypeScript
- SQLite
- Tailwind CSS
- shadcn/ui ou componentes equivalentes
- TanStack Query opcional
- Zustand opcional para estado de UI

Evitar servidor próprio na primeira versão.

O banco fica local:

```text
prospector.db
```

As chaves de API devem ser guardadas usando mecanismo seguro disponível no sistema operacional/Tauri, nunca hardcoded.

---

## 2.2 Separar descoberta de enriquecimento

Não solicitar dados caros/desnecessários durante toda busca.

Pipeline:

```text
DISCOVERY
    ↓
IDs + nome + localização + categoria
    ↓
DEDUPLICAÇÃO
    ↓
LEADS ÚNICOS
    ↓
ENRICHMENT
    ↓
website / telefone / rating / reviews / contatos públicos
```

Benefícios:

- menor custo;
- menor número de chamadas;
- melhor controle de jobs;
- facilidade de retry;
- leads podem ser enriquecidos apenas quando realmente interessantes.

---

## 2.3 Providers independentes

O software deve possuir uma abstração de providers.

Inicialmente:

```text
DiscoveryProvider
├── GooglePlacesProvider
└── GoogleMapsScraperProvider
```

Posteriormente poderão existir:

```text
BingProvider
OpenStreetMapProvider
CustomProvider
```

O domínio da aplicação não deve depender diretamente da implementação da Google Places API ou do scraper.

Interface conceitual:

```rust
trait DiscoveryProvider {
    async fn search(&self, request: SearchRequest) -> Result<Vec<DiscoveredPlace>>;
}
```

---

# 3. Fluxo principal do usuário

## 3.1 Criar busca

Usuário informa:

```text
Nicho: Dentist
Cidade: Miami, Florida
Raio: 30 km
```

Opcionalmente:

```text
Queries:
- dentist
- dental clinic
- orthodontist
- cosmetic dentist
```

Seleciona providers:

```text
[x] Google Places
[ ] Google Maps Scraper
```

Seleciona estratégia:

```text
Adaptive Coverage
```

Clica:

```text
Start Search
```

---

# 4. Motor de descoberta geográfica

## 4.1 Problema

Uma única consulta ampla não deve ser interpretada como uma listagem completa de todas as empresas de uma cidade.

Portanto, o aplicativo deve explorar o espaço geográfico sistematicamente.

---

## 4.2 Região inicial

O input do usuário:

```text
city + radius
```

deve ser convertido para:

```text
center_lat
center_lng
radius_meters
```

Em seguida gerar uma bounding box que contenha o círculo.

Modelo:

```rust
struct GeoRegion {
    north: f64,
    south: f64,
    east: f64,
    west: f64,
    depth: u8,
}
```

---

# 5. Quadtree adaptativa

A estratégia recomendada é uma **quadtree adaptativa**.

## 5.1 Primeira busca

Pesquisar a área.

Se os resultados estiverem claramente abaixo do limite/saturação da fonte:

```text
região = completa
```

Caso a consulta pareça saturada:

```text
subdividir em 4
```

Visual:

```text
┌──────────┬──────────┐
│ NW       │ NE       │
├──────────┼──────────┤
│ SW       │ SE       │
└──────────┴──────────┘
```

---

## 5.2 Recursão

Pseudo-código:

```rust
async fn search_region(region: GeoRegion, query: Query) {
    let results = provider.search(region, query).await?;

    persist_results(results);

    if should_split(region, results) {
        for child in region.split_into_four() {
            search_region(child, query).await?;
        }
    } else {
        mark_region_complete(region);
    }
}
```

---

# 6. Critério de subdivisão

Nunca depender exclusivamente de um número fixo.

Criar política configurável:

```rust
struct SplitPolicy {
    saturation_threshold: usize,
    min_cell_width_meters: f64,
    min_cell_height_meters: f64,
    max_depth: u8,
    min_new_unique_ratio: f64,
}
```

Critérios possíveis:

### A. Saturação

Se:

```text
returned_results >= saturation_threshold
```

há indício de truncamento.

### B. Profundidade máxima

Não subdividir além de:

```text
max_depth
```

### C. Tamanho mínimo

Não subdividir células menores que determinado tamanho.

### D. Ganho marginal

Se novas subdivisões repetidamente retornarem quase apenas leads já existentes, parar.

Exemplo:

```text
cell results: 50
new unique: 2
new_unique_ratio: 4%
```

Se isso se repetir, a área provavelmente está suficientemente explorada.

---

# 7. Coverage score

O sistema deve calcular um indicador interno de progresso.

Não chamar isso de "100% de todas as empresas existentes", pois não é possível garantir cobertura absoluta.

Usar:

```text
Exploration progress
```

ou:

```text
Search coverage
```

Baseado em células:

```text
completed_cells / discovered_cells
```

Pode-se mostrar:

```text
Coverage progress: 82%
```

Significado:

> 82% das células planejadas pelo algoritmo foram processadas.

Não significa que 82% dos estabelecimentos reais da cidade foram encontrados.

---

# 8. Query Packs

Criar agrupamentos de consultas por nicho.

Exemplo:

```json
{
  "id": "dentists",
  "label": "Dentists",
  "queries": [
    "dentist",
    "dental clinic",
    "orthodontist",
    "cosmetic dentist"
  ]
}
```

Cada query percorre a mesma área.

Todos os resultados passam pelo mesmo deduper.

---

# 9. Deduplicação

Deduplicação é componente crítico.

Prioridade:

## 9.1 Place ID

Quando disponível:

```text
provider + external_place_id
```

deve ser chave de identidade externa.

---

## 9.2 Fallback

Caso não exista ID confiável:

1. website normalizado;
2. telefone E.164;
3. nome + coordenadas próximas;
4. nome + endereço normalizado.

Criar:

```rust
struct LeadFingerprint {
    external_id: Option<String>,
    normalized_domain: Option<String>,
    normalized_phone: Option<String>,
    normalized_name: String,
    lat: Option<f64>,
    lng: Option<f64>,
}
```

Nunca unir automaticamente registros ambíguos com baixa confiança.

Criar:

```text
merge_confidence
```

---

# 10. Modelo de banco SQLite

## 10.1 leads

```sql
CREATE TABLE leads (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    canonical_name TEXT NOT NULL,

    category TEXT,
    address TEXT,

    latitude REAL,
    longitude REAL,

    phone TEXT,
    website TEXT,

    rating REAL,
    review_count INTEGER,

    email TEXT,
    instagram TEXT,
    facebook TEXT,
    whatsapp TEXT,

    website_status TEXT,

    lead_status TEXT NOT NULL DEFAULT 'new',

    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

---

## 10.2 external_places

```sql
CREATE TABLE external_places (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    lead_id INTEGER NOT NULL,

    provider TEXT NOT NULL,
    external_place_id TEXT,

    raw_name TEXT,
    raw_address TEXT,

    raw_payload_json TEXT,

    discovered_at TEXT NOT NULL,

    UNIQUE(provider, external_place_id),

    FOREIGN KEY(lead_id) REFERENCES leads(id)
);
```

---

## 10.3 search_jobs

```sql
CREATE TABLE search_jobs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    name TEXT,

    query_pack_id TEXT,
    city TEXT,

    center_lat REAL,
    center_lng REAL,
    radius_meters REAL,

    status TEXT,

    created_at TEXT NOT NULL,
    started_at TEXT,
    completed_at TEXT
);
```

Estados:

```text
pending
running
paused
completed
cancelled
failed
```

---

## 10.4 search_cells

```sql
CREATE TABLE search_cells (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    job_id INTEGER NOT NULL,

    parent_id INTEGER,

    north REAL NOT NULL,
    south REAL NOT NULL,
    east REAL NOT NULL,
    west REAL NOT NULL,

    depth INTEGER NOT NULL,

    status TEXT NOT NULL,

    raw_result_count INTEGER DEFAULT 0,
    new_unique_count INTEGER DEFAULT 0,

    query TEXT NOT NULL,
    provider TEXT NOT NULL,

    created_at TEXT NOT NULL,
    completed_at TEXT,

    FOREIGN KEY(job_id) REFERENCES search_jobs(id)
);
```

---

## 10.5 discoveries

```sql
CREATE TABLE discoveries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    lead_id INTEGER NOT NULL,
    job_id INTEGER NOT NULL,
    cell_id INTEGER,

    provider TEXT,
    query TEXT,

    discovered_at TEXT NOT NULL,

    FOREIGN KEY(lead_id) REFERENCES leads(id),
    FOREIGN KEY(job_id) REFERENCES search_jobs(id)
);
```

---

## 10.6 enrichment_jobs

```sql
CREATE TABLE enrichment_jobs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    lead_id INTEGER NOT NULL,

    enrichment_type TEXT,
    status TEXT,

    attempts INTEGER DEFAULT 0,

    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,

    FOREIGN KEY(lead_id) REFERENCES leads(id)
);
```

---

## 10.7 outreach_messages

```sql
CREATE TABLE outreach_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    lead_id INTEGER NOT NULL,

    channel TEXT NOT NULL,

    subject TEXT,
    message TEXT NOT NULL,

    status TEXT NOT NULL,

    created_at TEXT NOT NULL,
    approved_at TEXT,
    sent_at TEXT,

    FOREIGN KEY(lead_id) REFERENCES leads(id)
);
```

Status:

```text
draft
generated
approved
sent
replied
skipped
```

---

# 11. Enrichment Engine

Rodar depois da descoberta.

Pipeline:

```text
Lead
 ↓
Place Details
 ↓
Website crawler
 ↓
Contact extraction
 ↓
Social discovery
 ↓
Store enrichment
```

---

# 12. Website crawler

Ao existir website:

```text
https://empresa.com
```

visitar páginas públicas relevantes:

```text
/
/contact
/contact-us
/about
/about-us
```

Aplicar limite pequeno de páginas por domínio.

Extrair:

```text
mailto:
tel:
wa.me/
api.whatsapp.com/
instagram.com/
facebook.com/
linkedin.com/
```

Também pode detectar:

```text
formulário de contato
booking
chat widget
CMS
```

Não tentar contornar autenticação, CAPTCHA ou mecanismos anti-bot.

---

# 13. Website status

Criar campo:

```text
website_status
```

Valores:

```text
unknown
none
active
unreachable
redirect
parked
social_only
```

Isso permite filtros úteis:

```text
[x] sem website
[x] website inacessível
```

---

# 14. Filtros de leads

A tela principal de leads deve suportar filtros combináveis.

## Presença digital

```text
[ ] Sem website
[ ] Com website
[ ] Com Instagram
[ ] Sem Instagram
[ ] Com email
[ ] Com telefone
[ ] Com WhatsApp/link público
```

## Métricas

```text
Rating >= [4.0]
Reviews >= [20]
```

## Localização

```text
cidade
distância do centro
bairro
```

## Pipeline

```text
New
Qualified
Message Ready
Contacted
Replied
Interested
Meeting
Won
Lost
Skipped
```

---

# 15. Lead Score

Pode existir um score para priorização operacional, não como verdade absoluta.

Exemplo para serviço de criação de sites:

```text
+30 sem website
+15 website offline
+10 possui Instagram
+10 telefone presente
+10 email encontrado
+10 rating >= 4
+10 reviews >= 30
+5 negócio ativo/recentemente avaliado
```

Guardar os fatores separadamente:

```json
{
  "score": 75,
  "reasons": [
    "No website",
    "Instagram found",
    "Phone available",
    "Strong Google reviews"
  ]
}
```

O score deve ser configurável.

---

# 16. Mensagens personalizadas

Nunca tratar geração e envio como a mesma operação.

Pipeline:

```text
lead
 ↓
context builder
 ↓
message generator
 ↓
draft
 ↓
human review
 ↓
approved
 ↓
contact action
```

Contexto:

```json
{
  "name": "Miami Dental Care",
  "category": "Dentist",
  "city": "Miami",
  "website": null,
  "instagram": "@miamidental",
  "rating": 4.8,
  "reviews": 281
}
```

Mensagem pode mencionar somente fatos disponíveis.

Nunca inventar:

```text
nome do dono
receita
problemas internos
resultados comerciais
```

---

# 17. Outreach

O app deve priorizar controle humano.

Primeira versão:

```text
Generate Message
Copy Message
Open Email
Open Instagram
Open Website
Open WhatsApp link
Mark Contacted
```

Evitar construir inicialmente um sistema de blast automático.

Motivos:

- políticas de plataformas;
- anti-spam;
- reputação de domínio/conta;
- qualidade de mensagem;
- risco de bloqueio.

Arquitetura deve permitir integrações futuras compatíveis com políticas dos canais.

---

# 18. CRM simplificado

Estados:

```text
NEW
 ↓
QUALIFIED
 ↓
MESSAGE_READY
 ↓
CONTACTED
 ↓
REPLIED
 ↓
INTERESTED
 ↓
MEETING
 ↓
WON / LOST
```

Também:

```text
SKIPPED
DO_NOT_CONTACT
```

Cada alteração deve ir para um histórico.

---

# 19. UI principal

## 19.1 Dashboard

Mostrar:

```text
Total leads
New
Qualified
Contacted
Replied
Interested
Won
```

E:

```text
Leads without website
Leads with Instagram
Leads with email
```

---

# 20. Search page

Layout:

```text
┌──────────────────────────────────────────────────┐
│ New Search                                       │
├──────────────────────────────────────────────────┤
│ Niche        [ Dentists                       ]  │
│ Location     [ Miami, Florida                 ]  │
│ Radius       [ 30 km                         ]  │
│                                                  │
│ Queries                                          │
│ [dentist] [dental clinic] [orthodontist]        │
│                                                  │
│ Providers                                        │
│ [x] Google Places                                │
│ [ ] Google Maps Scraper                          │
│                                                  │
│                 [ Start Search ]                 │
└──────────────────────────────────────────────────┘
```

---

# 21. Map page

Mapa com células.

Estados visuais:

```text
pending
running
completed
saturated
split
failed
```

Ao clicar em uma célula:

```text
Query
Provider
Results
Unique results
Depth
Duration
```

---

# 22. Leads table

```text
┌────┬────────────────┬──────┬───────┬───────┬──────┬─────────┐
│    │ Business       │ Site │ IG    │ Email │ Rate │ Status  │
├────┼────────────────┼──────┼───────┼───────┼──────┼─────────┤
│ ☑  │ Miami Dental   │  —   │ ✓     │ ✓     │ 4.8  │ New     │
│ ☐  │ Smile Center   │ ✓    │ ✓     │ —     │ 4.3  │ New     │
└────┴────────────────┴──────┴───────┴───────┴──────┴─────────┘
```

Bulk actions:

```text
Enrich selected
Generate messages
Change status
Export CSV
```

---

# 23. Lead Detail

Seções:

```text
Overview
Contact
Digital Presence
Discovery Sources
Enrichment
Messages
Activity
```

Exibir:

```text
Found in 4 searches
Found by 2 providers
First discovered
Last refreshed
```

---

# 24. Rust modules

Estrutura sugerida:

```text
src-tauri/src/
│
├── main.rs
├── commands/
│   ├── search.rs
│   ├── leads.rs
│   ├── enrichment.rs
│   └── settings.rs
│
├── domain/
│   ├── lead.rs
│   ├── search_job.rs
│   ├── geo.rs
│   └── outreach.rs
│
├── providers/
│   ├── mod.rs
│   ├── google_places/
│   │   ├── client.rs
│   │   ├── text_search.rs
│   │   ├── nearby.rs
│   │   └── details.rs
│   │
│   └── google_maps_scraper/
│       ├── mod.rs
│       └── process.rs
│
├── discovery/
│   ├── engine.rs
│   ├── quadtree.rs
│   ├── scheduler.rs
│   └── policy.rs
│
├── dedupe/
│   ├── mod.rs
│   ├── fingerprint.rs
│   └── matcher.rs
│
├── enrichment/
│   ├── engine.rs
│   ├── website.rs
│   ├── contacts.rs
│   └── socials.rs
│
├── database/
│   ├── mod.rs
│   ├── migrations.rs
│   └── repositories/
│
└── services/
    ├── message_generator.rs
    └── lead_scoring.rs
```

---

# 25. Frontend modules

```text
src/
│
├── app/
├── components/
│   ├── leads/
│   ├── search/
│   ├── map/
│   └── ui/
│
├── pages/
│   ├── DashboardPage.tsx
│   ├── SearchPage.tsx
│   ├── SearchJobPage.tsx
│   ├── LeadsPage.tsx
│   ├── LeadDetailPage.tsx
│   └── SettingsPage.tsx
│
├── hooks/
├── stores/
├── lib/
└── types/
```

---

# 26. Jobs e concorrência

O Rust backend deve executar jobs controlados.

Nunca criar milhares de requests simultâneos.

Criar scheduler:

```rust
struct JobScheduler {
    discovery_concurrency: usize,
    enrichment_concurrency: usize,
}
```

Suportar:

```text
pause
resume
cancel
retry
```

Persistir jobs para que fechar o aplicativo não destrua o estado.

Ao reabrir:

```text
running → interrupted
```

e permitir resume.

---

# 27. Rate limiting

Cada provider deve possuir limiter próprio.

```rust
trait RateLimitedProvider {
    fn requests_per_second(&self) -> f64;
}
```

Suportar:

```text
exponential backoff
retry-after
max attempts
jitter
```

Não implementar mecanismos destinados a contornar CAPTCHA, bloqueio ou proteções anti-abuso.

---

# 28. Google Places

A integração deve suportar inicialmente:

```text
Text Search
Place Details
```

Nearby Search pode existir como provider/estratégia opcional.

Utilizar field masks mínimos.

Discovery:

```text
id
displayName
location
primaryType
```

Enrichment:

```text
websiteUri
nationalPhoneNumber
internationalPhoneNumber
rating
userRatingCount
formattedAddress
```

Os campos exatos devem ser confirmados na documentação atual no momento da implementação.

Não hardcodar premissas de preço.

---

# 29. Google Maps Scraper

O projeto:

```text
github.com/gosom/google-maps-scraper
```

deve ser tratado como provider independente.

Integração recomendada inicialmente:

```text
Tauri/Rust
    ↓
spawn subprocess
    ↓
google-maps-scraper
    ↓
JSON/CSV
    ↓
parser
    ↓
DiscoveredPlace
```

Não misturar os tipos internos do scraper com o domínio principal.

Criar adapter.

---

# 30. Normalização

Antes de persistir:

## URL

```text
https://www.example.com/
http://example.com
```

viram:

```text
example.com
```

para comparação.

## Telefone

Normalizar para formato internacional quando possível.

## Instagram

```text
https://instagram.com/company/
@company
```

viram:

```text
company
```

## Nomes

- lowercase para matching;
- remover excesso de espaços;
- preservar nome original para UI.

---

# 31. Exportação

Suportar:

```text
CSV
JSON
```

Campos selecionáveis.

Exemplo:

```text
name
category
address
phone
website
instagram
email
whatsapp
rating
review_count
status
```

---

# 32. Configurações

Página Settings:

```text
Google Places API key
LLM provider
LLM API key
Concurrency
Default radius
Default query packs
Data directory
```

Credenciais devem ser armazenadas de maneira segura.

---

# 33. Observabilidade

Criar logs estruturados.

Cada request pode registrar:

```text
timestamp
provider
job_id
cell_id
query
duration
status
result_count
```

Nunca logar API keys.

---

# 34. Erros

Classificar:

```text
NetworkError
RateLimitError
ProviderError
InvalidRequest
DatabaseError
ParseError
Cancelled
```

A UI deve mostrar mensagens compreensíveis.

---

# 35. Segurança

Nunca:

```text
hardcode API keys
exibir secrets no log
guardar secret no Git
```

Separar:

```text
application settings
secrets
```

---

# 36. Regras de dados e outreach

O aplicativo trabalha com dados comerciais públicos.

Ainda assim deve:

- respeitar termos das fontes;
- respeitar políticas dos canais;
- permitir marcar `DO_NOT_CONTACT`;
- não tentar contornar CAPTCHA;
- não tentar contornar rate limits;
- não automatizar envio em massa em canais que não permitam isso;
- manter revisão humana como padrão.

---

# 37. Fases de implementação

## Fase 1 — Foundation

Entregar:

- Tauri v2;
- React + TypeScript;
- SQLite;
- migrations;
- navegação;
- Settings;
- API key Google Places;
- estrutura de domínio.

Sem scraper.
Sem enrichment web.
Sem IA.

---

## Fase 2 — Places Discovery MVP

Entregar:

- Search page;
- cidade + raio;
- query;
- Google Places;
- primeira consulta;
- persistência de leads;
- deduplicação por Place ID;
- Leads table;
- filtros básicos.

Objetivo:

```text
Pesquisar uma cidade e obter uma lista local persistida.
```

---

## Fase 3 — Adaptive Geographic Search

Entregar:

- bounding box;
- search_cells;
- quadtree;
- subdivisão;
- scheduler;
- progress;
- pause/resume;
- visualização das células.

Objetivo:

```text
Aumentar cobertura sistematicamente.
```

---

## Fase 4 — Place Enrichment

Entregar:

- Place Details;
- telefone;
- website;
- rating;
- reviews;
- address;
- filtros.

---

## Fase 5 — Website Enrichment

Entregar:

- crawler limitado;
- email;
- Instagram;
- Facebook;
- WhatsApp links;
- website status.

---

## Fase 6 — Lead Qualification

Entregar:

- configurable lead score;
- saved filters;
- bulk selection;
- qualification queue.

---

## Fase 7 — Message Generator

Entregar:

- contexto do lead;
- provider de LLM configurável;
- templates;
- generated drafts;
- aprovação manual;
- copy/open actions.

---

## Fase 8 — Maps Scraper Provider

Entregar:

- integração subprocess;
- adapter;
- import;
- deduplicação cross-provider;
- métricas comparativas.

---

## Fase 9 — CRM

Entregar:

- pipeline;
- activity history;
- contacted/replied/interested/etc.;
- notes;
- follow-up dates.

---

# 38. Não fazer no MVP

Não implementar inicialmente:

- autenticação de usuários;
- multi-tenant;
- backend cloud;
- cobrança;
- campanhas gigantes;
- browser automation de Instagram;
- bypass de CAPTCHA;
- proxy rotation para evasão;
- envio massivo de WhatsApp;
- dezenas de providers;
- machine learning complexo.

O MVP precisa provar:

```text
SEARCH
 → DISCOVER
 → DEDUPE
 → FILTER
 → ENRICH
 → QUALIFY
 → MESSAGE
```

---

# 39. Primeira experiência ideal

Usuário abre.

```text
New Search
```

Preenche:

```text
Dentist
Miami, FL
30 km
```

Search.

Aplicativo encontra:

```text
60 initial results
```

Tabela mostra:

```text
15 without website
31 with website
40 with phone
18 with Instagram
```

Usuário filtra:

```text
No website
```

Obtém:

```text
15 leads
```

Seleciona 8.

```text
Enrich
```

Depois:

```text
Generate Messages
```

Revisa.

```text
Copy / Open contact channel / Mark contacted
```

Isso já entrega valor sem precisar descobrir milhares de empresas no primeiro dia.

---

# 40. Critério de sucesso do projeto

O projeto está funcionando quando o usuário consegue:

1. escolher nicho;
2. escolher cidade;
3. escolher raio;
4. buscar;
5. visualizar leads;
6. deduplicar;
7. filtrar por presença digital;
8. enriquecer;
9. gerar mensagem;
10. acompanhar status.

A busca geográfica adaptativa deve ser uma evolução do MVP, não uma dependência para o primeiro uso.

---

# 41. Regra de engenharia para o agente

Não tentar implementar todas as fases simultaneamente.

Para cada fase:

1. ler arquitetura;
2. identificar escopo;
3. propor arquivos afetados;
4. implementar;
5. rodar build;
6. rodar testes;
7. corrigir erros;
8. documentar decisão relevante;
9. somente então seguir.

Priorizar código simples, explícito e testável.

---

# 42. Nome interno sugerido

Nomes possíveis:

```text
LocalProspector
LeadAtlas
Prospector
LocalScout
LeadScout
```

Nome pode ser alterado sem impacto arquitetural.
