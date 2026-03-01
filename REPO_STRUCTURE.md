# Jovial — Repository Structure

```
jovial/
│
├── Cargo.toml                          # Workspace root
├── Cargo.lock
├── jovial.yaml.example              # Example project config for users
├── LICENSE
├── README.md
├── ARCHITECTURE.md                     # ASCII architecture diagrams
│
│
│   ╔══════════════════════════════════════════════════════════════╗
│   ║                     RUST CORE (Pass 2 + Plugins)           ║
│   ╚══════════════════════════════════════════════════════════════╝
│
├── crates/
│   │
│   ├── jovial-ast/                  # AST type definitions
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── java.rs                 # JavaNode enum (25 variants)
│   │       ├── go.rs                   # GoNode enum (output AST)
│   │       └── span.rs                 # Source location tracking
│   │
│   ├── jovial-manifest/             # Manifest types (from JVM extraction)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── beans.rs                # Bean, Dependency, ProxyInfo
│   │       ├── endpoints.rs            # Endpoint, EndpointParam
│   │       ├── entities.rs             # Entity, EntityField, Relationship
│   │       ├── advice.rs               # AdviceChain, TransactionSpec, CacheSpec
│   │       └── unresolved.rs           # Unresolved stubs
│   │
│   ├── jovial-plugin/               # Plugin trait + registry
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── traits.rs              # Plugin trait definition
│   │       ├── registry.rs            # Priority-ordered plugin registry
│   │       ├── context.rs             # MatchContext, TransformContext
│   │       ├── types.rs               # GoImport, GoDependency, ConfigValue
│   │       └── error.rs              # PluginError, Diagnostic, Severity
│   │
│   ├── jovial-walker/               # AST traversal engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── walker.rs              # Main walk loop + plugin dispatch
│   │       ├── default_convert.rs     # Mechanical Java→Go fallback
│   │       └── type_map.rs            # java_to_go_type(), operator mapping
│   │
│   ├── jovial-parser/               # Java source → JavaNode AST
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── lexer.rs               # Java tokenizer
│   │       ├── parser.rs              # Recursive descent parser
│   │       ├── resolver.rs            # Import resolution, FQCN lookup
│   │       └── type_resolver.rs       # TypeResolver trait impl
│   │
│   ├── jovial-emitter/              # GoNode AST → .go source files
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── emitter.rs             # Walk GoNode tree, emit strings
│   │       ├── formatter.rs           # Indentation, line wrapping
│   │       ├── imports.rs             # Import block deduplication
│   │       └── go_mod.rs             # go.mod generation from plugin deps
│   │
│   ├── jovial-codegen/              # High-level code generation orchestrator
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── project.rs             # Output project scaffold (main.go, etc.)
│   │       ├── wire.rs                # InitializeApp() DI wiring generator
│   │       ├── handlers.rs            # gin route handler generation
│   │       ├── models.rs              # GORM model struct generation
│   │       └── services.rs            # Service struct + tx wrapping
│   │
│   └── jovial-cli/                  # CLI binary
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs                # Entry point, arg parsing
│           ├── commands/
│           │   ├── mod.rs
│           │   ├── transpile.rs       # `jovial transpile` — main pipeline
│           │   ├── explain.rs         # `jovial explain <file>:<line>`
│           │   ├── install.rs         # `jovial install <plugin>`
│           │   ├── publish.rs         # `jovial publish ./plugin-dir`
│           │   ├── init_plugin.rs     # `jovial init-plugin --name=foo`
│           │   ├── test_plugin.rs     # `jovial test-plugin ./plugin-dir`
│           │   └── search.rs          # `jovial search "kafka"`
│           ├── config.rs              # jovial.yaml parsing
│           └── loader.rs              # Plugin discovery + registration
│
│
│   ╔══════════════════════════════════════════════════════════════╗
│   ║                  BUILT-IN PLUGINS                           ║
│   ╚══════════════════════════════════════════════════════════════╝
│
├── plugins/
│   │
│   ├── builtin/                        # Ship with the binary
│   │   │
│   │   ├── spring-web/                 # @RestController → gin-gonic/gin
│   │   │   ├── Cargo.toml
│   │   │   ├── jovial-plugin.yaml
│   │   │   └── src/
│   │   │       ├── lib.rs
│   │   │       ├── controller.rs      # Route handler transforms
│   │   │       ├── params.rs          # @PathVariable, @RequestParam, @RequestBody
│   │   │       └── response.rs        # ResponseEntity → gin.Context responses
│   │   │
│   │   ├── spring-data/                # JPA / Spring Data → GORM
│   │   │   ├── Cargo.toml
│   │   │   ├── jovial-plugin.yaml
│   │   │   └── src/
│   │   │       ├── lib.rs
│   │   │       ├── entity.rs          # @Entity → GORM model structs
│   │   │       ├── repository.rs      # CrudRepository → GORM queries
│   │   │       └── relationships.rs   # @OneToMany etc. → GORM associations
│   │   │
│   │   ├── spring-tx/                  # @Transactional → inline tx management
│   │   │   ├── Cargo.toml
│   │   │   ├── jovial-plugin.yaml
│   │   │   └── src/
│   │   │       └── lib.rs             # Weaves Begin/Commit/Rollback into body
│   │   │
│   │   ├── guava/                      # Google Guava → Go stdlib
│   │   │   ├── Cargo.toml
│   │   │   ├── jovial-plugin.yaml
│   │   │   └── src/
│   │   │       ├── lib.rs
│   │   │       ├── collections.rs     # ImmutableList/Map/Set
│   │   │       ├── preconditions.rs   # Preconditions.check*
│   │   │       ├── optional.rs        # Optional → *T / nil
│   │   │       └── strings.rs         # Strings utilities
│   │   │
│   │   ├── jackson/                    # Jackson → encoding/json
│   │   │   ├── Cargo.toml
│   │   │   ├── jovial-plugin.yaml
│   │   │   └── src/
│   │   │       ├── lib.rs
│   │   │       ├── annotations.rs     # @JsonProperty → struct tags
│   │   │       └── object_mapper.rs   # ObjectMapper → json.Marshal/Unmarshal
│   │   │
│   │   ├── lombok/                     # Lombok → plain Go structs
│   │   │   ├── Cargo.toml
│   │   │   ├── jovial-plugin.yaml
│   │   │   └── src/
│   │   │       └── lib.rs             # @Data, @Builder, @Getter/Setter → fields
│   │   │
│   │   └── slf4j/                      # SLF4J → log/slog
│   │       ├── Cargo.toml
│   │       ├── jovial-plugin.yaml
│   │       └── src/
│   │           └── lib.rs             # Logger.info() → slog.Info()
│   │
│   └── community/                      # Installed by `jovial install`
│       └── .gitkeep                    # (populated at runtime)
│
│
│   ╔══════════════════════════════════════════════════════════════╗
│   ║              JAVA EXTRACTOR (Pass 1 — JVM Oracle)          ║
│   ╚══════════════════════════════════════════════════════════════╝
│
├── extractor/
│   ├── pom.xml                         # Maven build (or build.gradle)
│   └── src/
│       └── main/
│           └── java/
│               └── com/jovial/extractor/
│                   ├── ManifestExtractor.java      # Entry point — boots Spring ctx
│                   ├── BeanExtractor.java           # Walk BeanFactory, resolve DI
│                   ├── EndpointExtractor.java        # RequestMappingHandlerMapping
│                   ├── EntityExtractor.java          # JPA metadata extraction
│                   ├── AdviceExtractor.java           # AOP proxy/advice chain walking
│                   ├── PropertyExtractor.java         # Environment property dump
│                   └── model/
│                       ├── ManifestModel.java         # Java POJOs matching manifest.rs
│                       ├── BeanModel.java
│                       ├── EndpointModel.java
│                       ├── EntityModel.java
│                       └── AdviceModel.java
│
│
│   ╔══════════════════════════════════════════════════════════════╗
│   ║                      TESTING                                ║
│   ╚══════════════════════════════════════════════════════════════╝
│
├── tests/
│   │
│   ├── fixtures/                       # Sample Java projects for e2e testing
│   │   │
│   │   ├── petclinic/                  # Spring Petclinic (classic demo app)
│   │   │   ├── src/                    # Java sources
│   │   │   ├── manifest.json           # Pre-extracted manifest
│   │   │   └── expected/               # Expected Go output (snapshot tests)
│   │   │       ├── main.go
│   │   │       ├── wire.go
│   │   │       ├── handlers/
│   │   │       ├── models/
│   │   │       └── services/
│   │   │
│   │   ├── order-service/              # Minimal CRUD service
│   │   │   ├── src/
│   │   │   ├── manifest.json
│   │   │   └── expected/
│   │   │
│   │   └── guava-heavy/                # Exercises Guava plugin transforms
│   │       ├── src/
│   │       ├── manifest.json
│   │       └── expected/
│   │
│   ├── plugin_tests/                   # Per-plugin snapshot tests
│   │   ├── guava_test.rs
│   │   ├── jackson_test.rs
│   │   ├── spring_web_test.rs
│   │   └── spring_data_test.rs
│   │
│   └── integration/                    # Full pipeline e2e tests
│       ├── transpile_test.rs           # Java → manifest → Go, then `go build`
│       └── explain_test.rs             # Verify transform audit trail
│
│
│   ╔══════════════════════════════════════════════════════════════╗
│   ║                     PLUGIN SDK & DOCS                       ║
│   ╚══════════════════════════════════════════════════════════════╝
│
├── plugin-sdk/
│   ├── template/                       # `jovial init-plugin` scaffold
│   │   ├── Cargo.toml.tmpl
│   │   ├── jovial-plugin.yaml.tmpl
│   │   ├── src/
│   │   │   └── lib.rs.tmpl             # Plugin trait boilerplate
│   │   └── testdata/
│   │       ├── input.java.tmpl
│   │       └── expected.go.tmpl
│   │
│   └── examples/                       # Reference implementations
│       ├── minimal/                    # Simplest possible plugin
│       │   ├── Cargo.toml
│       │   ├── jovial-plugin.yaml
│       │   ├── src/lib.rs
│       │   └── testdata/
│       │       ├── input.java
│       │       └── expected.go
│       │
│       └── apache-httpclient/          # Full-featured example
│           ├── Cargo.toml
│           ├── jovial-plugin.yaml
│           ├── src/
│           │   ├── lib.rs
│           │   ├── client.rs
│           │   └── request.rs
│           └── testdata/
│               ├── input.java
│               └── expected.go
│
│
│   ╔══════════════════════════════════════════════════════════════╗
│   ║                     REGISTRY                                ║
│   ╚══════════════════════════════════════════════════════════════╝
│
├── registry/
│   ├── index.json                      # Plugin index (name → repo → versions)
│   └── scripts/
│       ├── validate.sh                 # CI: validate plugin submissions
│       └── publish.sh                  # CI: add plugin to index
│
│
│   ╔══════════════════════════════════════════════════════════════╗
│   ║                     CI / INFRA                              ║
│   ╚══════════════════════════════════════════════════════════════╝
│
├── .github/
│   └── workflows/
│       ├── ci.yml                      # cargo test + clippy + fmt
│       ├── release.yml                 # Build binaries for linux/mac/windows
│       └── plugin-validation.yml       # Test community plugin submissions
│
├── Dockerfile                          # Builds CLI + bundles JVM extractor
└── Makefile                            # build, test, install, publish shortcuts


═══════════════════════════════════════════════════════════════════
                    DEPENDENCY GRAPH
═══════════════════════════════════════════════════════════════════


    jovial-cli
        │
        ├── jovial-codegen
        │       │
        │       ├── jovial-emitter
        │       │       │
        │       │       └── jovial-ast ◄──────────────────────┐
        │       │                                                 │
        │       ├── jovial-walker                             │
        │       │       │                                         │
        │       │       ├── jovial-plugin ──► jovial-ast   │
        │       │       │       │                                 │
        │       │       │       └── jovial-manifest            │
        │       │       │                                         │
        │       │       └── jovial-ast                        │
        │       │                                                 │
        │       └── jovial-manifest                           │
        │                                                         │
        ├── jovial-parser ──► jovial-ast                   │
        │                                                         │
        └── plugins/builtin/* ──► jovial-plugin ──► jovial-ast


    External (JVM):

    extractor/ (Java, Maven)
        │
        └── Outputs: manifest.json
              │
              └── Consumed by: jovial-manifest (serde::Deserialize)


═══════════════════════════════════════════════════════════════════
                    DATA FLOW THROUGH THE REPO
═══════════════════════════════════════════════════════════════════


    User's Java Project
         │
         │  jovial transpile
         │
         ▼
    ┌─────────────────────────────────────────────────────────┐
    │ jovial-cli                                           │
    │                                                         │
    │  1. Parse jovial.yaml              (config.rs)       │
    │  2. Load plugins                      (loader.rs)       │
    │                                                         │
    │  3. Run JVM extractor ──────────────► extractor/        │
    │     (spawns java process)              │                 │
    │                                        ▼                │
    │  4. Deserialize manifest.json ◄─── manifest.json        │
    │     (jovial-manifest)                                │
    │                                                         │
    │  5. Parse .java sources ────────────► jovial-parser   │
    │                                        │                 │
    │                                        ▼                │
    │  6. Walk AST + apply plugins ───────► jovial-walker   │
    │     (jovial-walker)                  │                 │
    │                                        ▼                │
    │  7. Emit Go source ─────────────────► jovial-emitter  │
    │                                        │                 │
    │  8. Scaffold project ───────────────► jovial-codegen  │
    │                                        │                 │
    │                                        ▼                │
    │  9. Write to output_dir ──► ./generated/                │
    │                                                         │
    └─────────────────────────────────────────────────────────┘
         │
         ▼
    Generated Go Project
    (go build → single binary)
```
