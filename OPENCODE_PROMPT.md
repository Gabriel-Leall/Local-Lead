# Prompt mestre para o OpenCode

Você está trabalhando em um aplicativo desktop chamado provisoriamente **Local Lead Prospector**.

Antes de alterar código, leia integralmente o arquivo:

```text
ARCHITECTURE.md
```

Ele é a especificação principal do projeto.

## Objetivo

Construir um aplicativo local-first em:

```text
Tauri v2
Rust
React
TypeScript
SQLite
Tailwind
```

para descoberta e organização de leads de empresas locais.

O usuário informa:

```text
nicho
cidade
raio
```

Exemplo:

```text
Dentist
Miami, Florida
30 km
```

O sistema consulta Google Places, persiste os negócios localmente, deduplica e permite filtrar os resultados.

Posteriormente haverá:

```text
adaptive geographic discovery
website enrichment
social/contact enrichment
lead scoring
message generation
Google Maps Scraper provider
mini CRM
```

Porém NÃO tente implementar tudo de uma vez.

---

# Modo de trabalho obrigatório

Trabalhe em fases pequenas e verificáveis.

Para cada fase:

1. inspecione o repositório atual;
2. entenda o que já existe;
3. compare com `ARCHITECTURE.md`;
4. descreva brevemente o que irá alterar;
5. implemente somente o escopo da fase;
6. rode formatter/linter;
7. rode testes;
8. rode build;
9. corrija todos os erros encontrados;
10. reporte arquivos modificados e decisões técnicas.

Não crie abstrações desnecessárias.

Não faça grandes refactors fora do escopo atual.

Não deixe TODOs críticos escondendo funcionalidades quebradas.

---

# FASE ATUAL: Foundation + Places Discovery MVP

Implemente somente:

## 1. Foundation

Configurar/confirmar:

```text
Tauri v2
React
TypeScript
SQLite
Tailwind
```

Criar estrutura limpa para frontend e Rust.

---

## 2. Navegação

Páginas:

```text
Dashboard
Search
Leads
Settings
```

Pode utilizar React Router.

---

## 3. Banco SQLite

Criar migrations e repositories para:

```text
leads
external_places
search_jobs
discoveries
```

Use o modelo de `ARCHITECTURE.md`, adaptando apenas quando necessário por razões técnicas.

Não criar todas as tabelas futuras ainda.

---

## 4. Settings

Criar tela onde usuário possa configurar:

```text
Google Places API Key
```

A chave não deve ser hardcoded nem commitada.

Utilize mecanismo apropriado do Tauri/OS para secret quando viável.

Se isso exigir complexidade excessiva para a primeira etapa, encapsule a leitura/escrita atrás de um `SecretStore` para podermos trocar a implementação depois.

Nunca imprimir a key no log.

---

## 5. Google Places provider

Criar uma abstração:

```rust
trait DiscoveryProvider
```

e implementar:

```text
GooglePlacesProvider
```

Nesta fase, usar Google Places para uma busca simples.

Separar client HTTP de lógica de domínio.

Usar field mask mínimo necessário para discovery.

Mapear resposta externa para:

```rust
DiscoveredPlace
```

Campos mínimos:

```text
external_id
name
category
latitude
longitude
address quando disponível
provider
```

Não espalhar structs do Google pelo restante da aplicação.

---

## 6. Search

Criar formulário:

```text
Niche / Query
City
Radius
```

Nesta primeira fase, se geocodificação da cidade ainda não estiver implementada, é aceitável começar com uma estratégia simples documentada, MAS a arquitetura deve permitir adicionar geocoding e quadtree depois.

Preferência:

```text
query + city
```

via Text Search.

O raio pode inicialmente ser persistido como intenção de busca mesmo que ainda não seja usado pelo algoritmo adaptativo.

Não fingir que a consulta inicial representa cobertura completa.

Na UI chamar resultado de:

```text
Initial search results
```

---

## 7. Persistência

Ao obter resultados:

1. iniciar search_job;
2. converter response para domain;
3. deduplicar pelo external place ID;
4. criar/atualizar lead;
5. registrar external_place;
6. registrar discovery;
7. finalizar job.

A busca repetida não deve criar o mesmo lead várias vezes.

---

## 8. Leads

Criar tabela:

```text
Business
Category
Address
Website
Phone
Rating
Reviews
Status
```

Campos ainda não enriquecidos podem aparecer como:

```text
—
```

Adicionar filtros iniciais:

```text
search by name
category
status
```

Não implementar Instagram/website filtering antes de existirem dados de enrichment.

---

## 9. Estados do lead

Nesta fase:

```text
new
qualified
skipped
```

Projetar para expansão futura.

---

## 10. Dashboard

Mostrar no mínimo:

```text
Total leads
New leads
Searches completed
```

Os valores devem vir do banco real, não mocks permanentes.

---

# Fora do escopo desta fase

NÃO implementar ainda:

```text
quadtree
grid
adaptive splitting
website crawling
email extraction
Instagram crawling
WhatsApp automation
Google Maps scraper
LLM
message generation
email sending
CRM completo
proxy rotation
CAPTCHA bypass
bulk outreach
```

Não tente antecipar essas features.

Deixe interfaces limpas para adicioná-las depois.

---

# Regras importantes

## API

Nunca hardcodar API key.

Nunca logar segredo.

Tratar:

```text
HTTP errors
invalid API key
rate limit
empty results
network timeout
JSON parse failures
```

Mostrar erro compreensível na UI.

---

## Banco

Use migrations.

Evite SQL espalhado por commands Tauri.

Criar repositories.

---

## Rust

Preferir:

```text
domain
providers
database
services
commands
```

Evitar `main.rs` gigante.

---

## Frontend

Separar:

```text
pages
components
hooks
lib
types
```

Componentes devem ser pequenos.

---

# UX

O fluxo mínimo deve funcionar assim:

```text
Settings
 ↓
save Google Places key

Search
 ↓
Dentist
Miami, Florida
30 km
 ↓
Search

Results persisted

Leads
 ↓
table of businesses

Dashboard
 ↓
updated counts
```

---

# Testes mínimos

Adicionar testes para:

```text
deduplication
provider response mapping
lead repository upsert
```

Se possível:

```text
SQLite in-memory test
```

Não fazer chamadas reais à API durante testes.

Mocks/fixtures são obrigatórios para provider tests.

---

# Definition of Done da fase

A fase termina somente quando:

- app inicia;
- navegação funciona;
- migration roda;
- Settings salva API key;
- Search chama Google Places;
- resposta é mapeada;
- leads são persistidos;
- busca repetida não duplica;
- Leads page lê banco;
- Dashboard lê banco;
- build do frontend passa;
- build do Tauri/Rust passa;
- testes passam.

Quando terminar, NÃO comece automaticamente a próxima fase.

Pare e apresente:

```text
Implemented
Files changed
Architecture decisions
Known limitations
How to run
How to test
Recommended next phase
```

A próxima fase será **Adaptive Geographic Search**, mas só deve ser iniciada após nova instrução.
