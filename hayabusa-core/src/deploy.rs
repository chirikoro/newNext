//! Multi-Platform Deploy Adapters for Hayabusa.
//!
//! Generate deployment configurations for various platforms:
//! - **Fly.io** — Edge deployment with global regions
//! - **Railway** — One-click Rust deployment
//! - **Docker** — Universal containerized deployment
//! - **AWS (ECS/Lambda)** — Serverless or container-based
//! - **Cloudflare Workers** — Edge WASM deployment (config only)
//!
//! ## Usage
//! ```ignore
//! use hayabusa_core::prelude::*;
//!
//! // Fly.io
//! let fly = FlyIoAdapter::new("my-hayabusa-app")
//!     .region("nrt")  // Tokyo
//!     .memory(256)
//!     .min_machines(1);
//! println!("{}", fly.generate_fly_toml());
//!
//! // Docker
//! let docker = DockerAdapter::new("my-app");
//! println!("{}", docker.generate_dockerfile());
//!
//! // Railway
//! let railway = RailwayAdapter::new();
//! println!("{}", railway.generate_config());
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ─── Fly.io Adapter ─────────────────────────────────────────

/// Fly.io deployment adapter
///
/// Fly.io is the best match for Hayabusa:
/// - Native Docker/binary support
/// - Global edge deployment (30+ regions)
/// - Built-in TLS, load balancing
/// - Persistent volumes for ISR cache
/// - Machines API for auto-scaling
#[derive(Debug, Clone)]
pub struct FlyIoAdapter {
    /// App name on Fly.io
    pub app_name: String,
    /// Primary region (e.g., "nrt" for Tokyo, "iad" for Virginia)
    pub primary_region: String,
    /// Additional regions for edge deployment
    pub regions: Vec<String>,
    /// Memory in MB per machine (default: 256)
    pub memory: u32,
    /// CPU kind: "shared" or "performance"
    pub cpu_kind: String,
    /// Number of CPUs
    pub cpus: u32,
    /// Minimum machines (auto-scale floor)
    pub min_machines: u32,
    /// Maximum machines (auto-scale ceiling)
    pub max_machines: u32,
    /// Internal port the app listens on
    pub internal_port: u16,
    /// Enable persistent volume for ISR cache
    pub volume: Option<FlyVolume>,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Health check path
    pub health_check_path: String,
    /// Auto-stop idle machines
    pub auto_stop: bool,
    /// Auto-start on request
    pub auto_start: bool,
    /// Concurrency limits
    pub concurrency_soft_limit: u32,
    pub concurrency_hard_limit: u32,
}

/// Fly.io persistent volume configuration
#[derive(Debug, Clone)]
pub struct FlyVolume {
    pub name: String,
    pub size_gb: u32,
    pub mount_path: String,
}

impl FlyIoAdapter {
    pub fn new(app_name: impl Into<String>) -> Self {
        Self {
            app_name: app_name.into(),
            primary_region: "nrt".to_string(),
            regions: Vec::new(),
            memory: 256,
            cpu_kind: "shared".to_string(),
            cpus: 1,
            min_machines: 1,
            max_machines: 3,
            internal_port: 3000,
            volume: None,
            env: HashMap::new(),
            health_check_path: "/".to_string(),
            auto_stop: true,
            auto_start: true,
            concurrency_soft_limit: 200,
            concurrency_hard_limit: 250,
        }
    }

    pub fn region(mut self, region: impl Into<String>) -> Self {
        self.primary_region = region.into();
        self
    }

    pub fn add_region(mut self, region: impl Into<String>) -> Self {
        self.regions.push(region.into());
        self
    }

    pub fn memory(mut self, mb: u32) -> Self {
        self.memory = mb;
        self
    }

    pub fn min_machines(mut self, n: u32) -> Self {
        self.min_machines = n;
        self
    }

    pub fn max_machines(mut self, n: u32) -> Self {
        self.max_machines = n;
        self
    }

    pub fn internal_port(mut self, port: u16) -> Self {
        self.internal_port = port;
        self
    }

    pub fn volume(mut self, name: impl Into<String>, size_gb: u32, mount: impl Into<String>) -> Self {
        self.volume = Some(FlyVolume {
            name: name.into(),
            size_gb,
            mount_path: mount.into(),
        });
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    pub fn performance_cpu(mut self, cpus: u32) -> Self {
        self.cpu_kind = "performance".to_string();
        self.cpus = cpus;
        self
    }

    /// Generate `fly.toml` configuration file
    pub fn generate_fly_toml(&self) -> String {
        let mut toml = String::new();

        toml.push_str(&format!("app = \"{}\"\n", self.app_name));
        toml.push_str(&format!("primary_region = \"{}\"\n", self.primary_region));
        toml.push_str("kill_signal = \"SIGINT\"\n");
        toml.push_str("kill_timeout = \"5s\"\n");

        // Build
        toml.push_str("\n[build]\n");
        toml.push_str("  dockerfile = \"Dockerfile\"\n");

        // Environment variables
        if !self.env.is_empty() {
            toml.push_str("\n[env]\n");
            for (k, v) in &self.env {
                toml.push_str(&format!("  {} = \"{}\"\n", k, v));
            }
        }

        // HTTP service
        toml.push_str("\n[http_service]\n");
        toml.push_str(&format!("  internal_port = {}\n", self.internal_port));
        toml.push_str("  force_https = true\n");
        toml.push_str(&format!("  auto_stop_machines = {}\n", self.auto_stop));
        toml.push_str(&format!("  auto_start_machines = {}\n", self.auto_start));
        toml.push_str(&format!("  min_machines_running = {}\n", self.min_machines));

        // Concurrency
        toml.push_str("\n  [http_service.concurrency]\n");
        toml.push_str("    type = \"requests\"\n");
        toml.push_str(&format!("    soft_limit = {}\n", self.concurrency_soft_limit));
        toml.push_str(&format!("    hard_limit = {}\n", self.concurrency_hard_limit));

        // Health checks
        toml.push_str("\n[[http_service.checks]]\n");
        toml.push_str("  grace_period = \"5s\"\n");
        toml.push_str("  interval = \"15s\"\n");
        toml.push_str("  method = \"GET\"\n");
        toml.push_str(&format!("  path = \"{}\"\n", self.health_check_path));
        toml.push_str("  timeout = \"2s\"\n");

        // VM resources
        toml.push_str("\n[[vm]]\n");
        toml.push_str(&format!("  memory = \"{}mb\"\n", self.memory));
        toml.push_str(&format!("  cpu_kind = \"{}\"\n", self.cpu_kind));
        toml.push_str(&format!("  cpus = {}\n", self.cpus));

        // Volume mount
        if let Some(ref vol) = self.volume {
            toml.push_str("\n[[mounts]]\n");
            toml.push_str(&format!("  source = \"{}\"\n", vol.name));
            toml.push_str(&format!("  destination = \"{}\"\n", vol.mount_path));
        }

        // Statics (served by Fly's CDN)
        toml.push_str("\n[[statics]]\n");
        toml.push_str("  guest_path = \"/app/public\"\n");
        toml.push_str("  url_prefix = \"/static\"\n");

        toml
    }

    /// Generate Fly.io deploy commands
    pub fn deploy_commands(&self) -> Vec<String> {
        let mut cmds = vec![
            format!("fly apps create {}", self.app_name),
        ];

        // Create volume if configured
        if let Some(ref vol) = self.volume {
            cmds.push(format!(
                "fly volumes create {} --size {} --region {}",
                vol.name, vol.size_gb, self.primary_region
            ));
        }

        // Set secrets (env vars)
        for (k, v) in &self.env {
            cmds.push(format!("fly secrets set {}={}", k, v));
        }

        // Deploy
        cmds.push("fly deploy".to_string());

        // Scale to additional regions
        for region in &self.regions {
            cmds.push(format!("fly scale count {} --region {}", self.min_machines, region));
        }

        cmds
    }
}

// ─── Docker Adapter ─────────────────────────────────────────

/// Universal Docker deployment adapter
///
/// Generates an optimized multi-stage Dockerfile:
/// 1. Build stage: Rust compilation with cargo-chef for layer caching
/// 2. Runtime stage: Minimal distroless/scratch image (~10MB)
#[derive(Debug, Clone)]
pub struct DockerAdapter {
    /// Binary name
    pub binary_name: String,
    /// Rust version for builder image
    pub rust_version: String,
    /// Base runtime image
    pub runtime_image: String,
    /// Exposed port
    pub port: u16,
    /// Extra system packages needed at runtime
    pub runtime_packages: Vec<String>,
    /// Use cargo-chef for faster rebuilds
    pub use_cargo_chef: bool,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Extra files to copy to runtime
    pub copy_dirs: Vec<(String, String)>,
}

impl DockerAdapter {
    pub fn new(binary_name: impl Into<String>) -> Self {
        Self {
            binary_name: binary_name.into(),
            rust_version: "1.82".to_string(),
            runtime_image: "debian:bookworm-slim".to_string(),
            port: 3000,
            runtime_packages: Vec::new(),
            use_cargo_chef: true,
            env: HashMap::new(),
            copy_dirs: vec![
                ("public".to_string(), "/app/public".to_string()),
                ("templates".to_string(), "/app/templates".to_string()),
            ],
        }
    }

    pub fn rust_version(mut self, version: impl Into<String>) -> Self {
        self.rust_version = version.into();
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn runtime_package(mut self, pkg: impl Into<String>) -> Self {
        self.runtime_packages.push(pkg.into());
        self
    }

    pub fn copy_dir(mut self, src: impl Into<String>, dest: impl Into<String>) -> Self {
        self.copy_dirs.push((src.into(), dest.into()));
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    pub fn scratch_runtime(mut self) -> Self {
        self.runtime_image = "scratch".to_string();
        self
    }

    /// Generate an optimized multi-stage Dockerfile
    pub fn generate_dockerfile(&self) -> String {
        let mut df = String::new();

        // ── Stage 1: cargo-chef planner (dependency caching)
        if self.use_cargo_chef {
            df.push_str(&format!(
                "# Stage 1: Plan dependencies\nFROM rust:{} AS chef\n", self.rust_version
            ));
            df.push_str("RUN cargo install cargo-chef\n");
            df.push_str("WORKDIR /app\n\n");

            df.push_str("FROM chef AS planner\n");
            df.push_str("COPY . .\n");
            df.push_str("RUN cargo chef prepare --recipe-path recipe.json\n\n");

            // ── Stage 2: Build with cached deps
            df.push_str("# Stage 2: Build with cached dependencies\n");
            df.push_str("FROM chef AS builder\n");
            df.push_str("COPY --from=planner /app/recipe.json recipe.json\n");
            df.push_str("RUN cargo chef cook --release --recipe-path recipe.json\n");
            df.push_str("COPY . .\n");
            df.push_str(&format!(
                "RUN cargo build --release --bin {}\n\n",
                self.binary_name
            ));
        } else {
            df.push_str(&format!(
                "# Stage 1: Build\nFROM rust:{} AS builder\n", self.rust_version
            ));
            df.push_str("WORKDIR /app\n");
            df.push_str("COPY . .\n");
            df.push_str(&format!(
                "RUN cargo build --release --bin {}\n\n",
                self.binary_name
            ));
        }

        // ── Stage 3: Minimal runtime
        df.push_str(&format!(
            "# Stage 3: Minimal runtime\nFROM {}\n",
            self.runtime_image
        ));

        // Install runtime packages
        if !self.runtime_packages.is_empty()
            && self.runtime_image != "scratch"
        {
            df.push_str(&format!(
                "RUN apt-get update && apt-get install -y {} && rm -rf /var/lib/apt/lists/*\n",
                self.runtime_packages.join(" ")
            ));
        }

        df.push_str("WORKDIR /app\n\n");

        // Copy binary
        df.push_str(&format!(
            "COPY --from=builder /app/target/release/{} /app/{}\n",
            self.binary_name, self.binary_name
        ));

        // Copy additional directories
        for (src, dest) in &self.copy_dirs {
            df.push_str(&format!("COPY {} {}\n", src, dest));
        }

        df.push('\n');

        // Environment variables
        for (k, v) in &self.env {
            df.push_str(&format!("ENV {}={}\n", k, v));
        }
        df.push_str(&format!("ENV PORT={}\n", self.port));

        df.push_str(&format!("\nEXPOSE {}\n", self.port));
        df.push_str(&format!(
            "CMD [\"/app/{}\"]\n",
            self.binary_name
        ));

        df
    }

    /// Generate .dockerignore
    pub fn generate_dockerignore() -> &'static str {
        "target/\n\
         .git/\n\
         .gitignore\n\
         .env\n\
         .env.local\n\
         *.md\n\
         Dockerfile\n\
         .dockerignore\n\
         .vercel/\n\
         .fly/\n\
         node_modules/\n"
    }

    /// Generate docker-compose.yml for local dev with hot reload
    pub fn generate_compose(&self, with_postgres: bool, with_redis: bool) -> String {
        let mut yml = String::from("version: '3.8'\n\nservices:\n");

        // App service
        yml.push_str("  app:\n");
        yml.push_str("    build: .\n");
        yml.push_str(&format!("    ports:\n      - \"{}:{}\"\n", self.port, self.port));
        yml.push_str("    volumes:\n      - .:/app\n");
        yml.push_str("    environment:\n");
        yml.push_str(&format!("      - PORT={}\n", self.port));
        yml.push_str("      - RUST_LOG=info\n");

        if with_postgres {
            yml.push_str("      - DATABASE_URL=postgres://hayabusa:hayabusa@db:5432/hayabusa\n");
            yml.push_str("    depends_on:\n      - db\n");
        }
        if with_redis {
            yml.push_str("      - REDIS_URL=redis://redis:6379\n");
            if !with_postgres {
                yml.push_str("    depends_on:\n");
            }
            yml.push_str("      - redis\n");
        }

        // Postgres
        if with_postgres {
            yml.push_str("\n  db:\n");
            yml.push_str("    image: postgres:16-alpine\n");
            yml.push_str("    environment:\n");
            yml.push_str("      POSTGRES_USER: hayabusa\n");
            yml.push_str("      POSTGRES_PASSWORD: hayabusa\n");
            yml.push_str("      POSTGRES_DB: hayabusa\n");
            yml.push_str("    ports:\n      - \"5432:5432\"\n");
            yml.push_str("    volumes:\n      - pgdata:/var/lib/postgresql/data\n");
        }

        // Redis
        if with_redis {
            yml.push_str("\n  redis:\n");
            yml.push_str("    image: redis:7-alpine\n");
            yml.push_str("    ports:\n      - \"6379:6379\"\n");
        }

        // Volumes
        if with_postgres {
            yml.push_str("\nvolumes:\n  pgdata:\n");
        }

        yml
    }
}

// ─── Railway Adapter ────────────────────────────────────────

/// Railway deployment adapter
///
/// Railway auto-detects Rust projects via Cargo.toml and builds them.
/// This generates a `railway.toml` for fine-tuning the deployment.
#[derive(Debug, Clone)]
pub struct RailwayAdapter {
    /// Build command
    pub build_command: String,
    /// Start command
    pub start_command: String,
    /// Watch patterns for auto-redeploy
    pub watch_patterns: Vec<String>,
    /// Health check path
    pub health_check_path: Option<String>,
    /// Number of replicas
    pub replicas: u32,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Restart policy: "always", "on-failure", "never"
    pub restart_policy: String,
}

impl Default for RailwayAdapter {
    fn default() -> Self {
        Self {
            build_command: "cargo build --release".to_string(),
            start_command: "./target/release/example-app".to_string(),
            watch_patterns: vec![
                "src/**".to_string(),
                "Cargo.toml".to_string(),
                "Cargo.lock".to_string(),
            ],
            health_check_path: Some("/".to_string()),
            replicas: 1,
            env: HashMap::new(),
            restart_policy: "on-failure".to_string(),
        }
    }
}

impl RailwayAdapter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start_command(mut self, cmd: impl Into<String>) -> Self {
        self.start_command = cmd.into();
        self
    }

    pub fn health_check(mut self, path: impl Into<String>) -> Self {
        self.health_check_path = Some(path.into());
        self
    }

    pub fn replicas(mut self, n: u32) -> Self {
        self.replicas = n;
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Generate `railway.toml`
    pub fn generate_config(&self) -> String {
        let mut toml = String::new();

        toml.push_str("[build]\n");
        toml.push_str(&format!("builder = \"nixpacks\"\n"));
        toml.push_str(&format!("buildCommand = \"{}\"\n", self.build_command));

        let watch = self
            .watch_patterns
            .iter()
            .map(|w| format!("\"{}\"", w))
            .collect::<Vec<_>>()
            .join(", ");
        toml.push_str(&format!("watchPatterns = [{}]\n", watch));

        toml.push_str("\n[deploy]\n");
        toml.push_str(&format!("startCommand = \"{}\"\n", self.start_command));
        toml.push_str(&format!("restartPolicyType = \"{}\"\n", self.restart_policy));
        toml.push_str(&format!("numReplicas = {}\n", self.replicas));

        if let Some(ref path) = self.health_check_path {
            toml.push_str(&format!("healthcheckPath = \"{}\"\n", path));
            toml.push_str("healthcheckTimeout = 5\n");
        }

        toml
    }

    /// Generate `nixpacks.toml` for Railway's Nixpacks builder
    pub fn generate_nixpacks(&self) -> String {
        let mut toml = String::new();

        toml.push_str("[phases.setup]\n");
        toml.push_str("nixPkgs = [\"rustc\", \"cargo\", \"gcc\", \"pkg-config\", \"openssl\"]\n");

        toml.push_str("\n[phases.build]\n");
        toml.push_str(&format!("cmds = [\"{}\"]\n", self.build_command));

        toml.push_str("\n[start]\n");
        toml.push_str(&format!("cmd = \"{}\"\n", self.start_command));

        toml
    }
}

// ─── AWS Adapter ────────────────────────────────────────────

/// AWS deployment adapter (ECS Fargate / Lambda)
#[derive(Debug, Clone)]
pub struct AwsAdapter {
    /// Application name
    pub app_name: String,
    /// AWS region
    pub region: String,
    /// Deployment mode
    pub mode: AwsDeployMode,
    /// Memory in MB
    pub memory: u32,
    /// CPU units (1024 = 1 vCPU)
    pub cpu: u32,
    /// Container port
    pub port: u16,
    /// Environment variables
    pub env: HashMap<String, String>,
}

/// AWS deployment modes
#[derive(Debug, Clone, PartialEq)]
pub enum AwsDeployMode {
    /// ECS Fargate (long-running container)
    EcsFargate,
    /// Lambda with custom runtime (serverless)
    Lambda,
}

impl AwsAdapter {
    pub fn ecs(app_name: impl Into<String>, region: impl Into<String>) -> Self {
        Self {
            app_name: app_name.into(),
            region: region.into(),
            mode: AwsDeployMode::EcsFargate,
            memory: 512,
            cpu: 256,
            port: 3000,
            env: HashMap::new(),
        }
    }

    pub fn lambda(app_name: impl Into<String>, region: impl Into<String>) -> Self {
        Self {
            app_name: app_name.into(),
            region: region.into(),
            mode: AwsDeployMode::Lambda,
            memory: 256,
            cpu: 0,
            port: 3000,
            env: HashMap::new(),
        }
    }

    /// Generate ECS task definition JSON
    pub fn generate_task_definition(&self) -> String {
        let env_entries: Vec<String> = self
            .env
            .iter()
            .map(|(k, v)| {
                format!(
                    "        {{ \"name\": \"{}\", \"value\": \"{}\" }}",
                    k, v
                )
            })
            .collect();

        let env_json = if env_entries.is_empty() {
            "[]".to_string()
        } else {
            format!("[\n{}\n      ]", env_entries.join(",\n"))
        };

        format!(
            r#"{{
  "family": "{name}",
  "networkMode": "awsvpc",
  "requiresCompatibilities": ["FARGATE"],
  "cpu": "{cpu}",
  "memory": "{memory}",
  "containerDefinitions": [
    {{
      "name": "{name}",
      "image": "{{{{.ImageURI}}}}",
      "portMappings": [
        {{
          "containerPort": {port},
          "protocol": "tcp"
        }}
      ],
      "environment": {env},
      "logConfiguration": {{
        "logDriver": "awslogs",
        "options": {{
          "awslogs-group": "/ecs/{name}",
          "awslogs-region": "{region}",
          "awslogs-stream-prefix": "ecs"
        }}
      }},
      "healthCheck": {{
        "command": ["CMD-SHELL", "curl -f http://localhost:{port}/ || exit 1"],
        "interval": 30,
        "timeout": 5,
        "retries": 3,
        "startPeriod": 10
      }}
    }}
  ]
}}"#,
            name = self.app_name,
            cpu = self.cpu,
            memory = self.memory,
            port = self.port,
            region = self.region,
            env = env_json,
        )
    }

    /// Generate AWS SAM template for Lambda deployment
    pub fn generate_sam_template(&self) -> String {
        format!(
            r#"AWSTemplateFormatVersion: '2010-09-09'
Transform: AWS::Serverless-2016-10-31
Description: Hayabusa app - {name}

Globals:
  Function:
    Timeout: 30
    MemorySize: {memory}

Resources:
  HayabusaFunction:
    Type: AWS::Serverless::Function
    Properties:
      FunctionName: {name}
      Runtime: provided.al2
      Handler: bootstrap
      CodeUri: ./target/release/
      Architectures:
        - x86_64
      Events:
        ApiGateway:
          Type: HttpApi
          Properties:
            Path: /{{proxy+}}
            Method: ANY
        Root:
          Type: HttpApi
          Properties:
            Path: /
            Method: ANY

Outputs:
  ApiUrl:
    Description: API Gateway URL
    Value: !Sub "https://${{ServerlessHttpApi}}.execute-api.${{AWS::Region}}.amazonaws.com/"
"#,
            name = self.app_name,
            memory = self.memory,
        )
    }
}

// ─── Cloudflare Workers Adapter ─────────────────────────────

/// Cloudflare Workers adapter (WASM-based edge deployment)
///
/// Note: Full Hayabusa requires compiling to `wasm32-wasi` target.
/// This generates the wrangler.toml configuration.
#[derive(Debug, Clone)]
pub struct CloudflareAdapter {
    pub name: String,
    pub route: Option<String>,
    pub compatibility_date: String,
    pub kv_namespaces: Vec<(String, String)>,
    pub vars: HashMap<String, String>,
}

impl CloudflareAdapter {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            route: None,
            compatibility_date: "2024-12-01".to_string(),
            kv_namespaces: Vec::new(),
            vars: HashMap::new(),
        }
    }

    pub fn route(mut self, route: impl Into<String>) -> Self {
        self.route = Some(route.into());
        self
    }

    pub fn kv(mut self, binding: impl Into<String>, id: impl Into<String>) -> Self {
        self.kv_namespaces.push((binding.into(), id.into()));
        self
    }

    pub fn var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.vars.insert(key.into(), value.into());
        self
    }

    /// Generate `wrangler.toml`
    pub fn generate_wrangler_toml(&self) -> String {
        let mut toml = String::new();

        toml.push_str(&format!("name = \"{}\"\n", self.name));
        toml.push_str("main = \"build/worker/shim.mjs\"\n");
        toml.push_str(&format!(
            "compatibility_date = \"{}\"\n",
            self.compatibility_date
        ));

        if let Some(ref route) = self.route {
            toml.push_str(&format!("route = \"{}\"\n", route));
        }

        // WASM build
        toml.push_str("\n[build]\n");
        toml.push_str("command = \"cargo install -q worker-build && worker-build --release\"\n");

        // KV namespaces
        for (binding, id) in &self.kv_namespaces {
            toml.push_str(&format!(
                "\n[[kv_namespaces]]\nbinding = \"{}\"\nid = \"{}\"\n",
                binding, id
            ));
        }

        // Variables
        if !self.vars.is_empty() {
            toml.push_str("\n[vars]\n");
            for (k, v) in &self.vars {
                toml.push_str(&format!("{} = \"{}\"\n", k, v));
            }
        }

        toml
    }
}

// ─── Deploy Config Generator ────────────────────────────────

/// High-level deploy config generator that creates configs for all platforms
pub struct DeployGenerator {
    project_dir: PathBuf,
    binary_name: String,
    port: u16,
}

impl DeployGenerator {
    pub fn new(project_dir: impl Into<PathBuf>, binary_name: impl Into<String>) -> Self {
        Self {
            project_dir: project_dir.into(),
            binary_name: binary_name.into(),
            port: 3000,
        }
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Write Dockerfile and .dockerignore to the project directory
    pub fn write_docker(&self) -> Result<(), std::io::Error> {
        let docker = DockerAdapter::new(&self.binary_name).port(self.port);
        std::fs::write(
            self.project_dir.join("Dockerfile"),
            docker.generate_dockerfile(),
        )?;
        std::fs::write(
            self.project_dir.join(".dockerignore"),
            DockerAdapter::generate_dockerignore(),
        )?;
        Ok(())
    }

    /// Write fly.toml to the project directory
    pub fn write_fly_toml(&self, app_name: &str) -> Result<(), std::io::Error> {
        let fly = FlyIoAdapter::new(app_name).internal_port(self.port);
        std::fs::write(
            self.project_dir.join("fly.toml"),
            fly.generate_fly_toml(),
        )
    }

    /// Write railway.toml to the project directory
    pub fn write_railway_toml(&self) -> Result<(), std::io::Error> {
        let railway = RailwayAdapter::new()
            .start_command(format!("./target/release/{}", self.binary_name));
        std::fs::write(
            self.project_dir.join("railway.toml"),
            railway.generate_config(),
        )
    }
}

// ─── Helper: write file ─────────────────────────────────────

#[allow(dead_code)]
fn write_file(path: &Path, content: &str) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Fly.io tests ──

    #[test]
    fn test_fly_default() {
        let fly = FlyIoAdapter::new("my-app");
        assert_eq!(fly.app_name, "my-app");
        assert_eq!(fly.primary_region, "nrt");
        assert_eq!(fly.memory, 256);
    }

    #[test]
    fn test_fly_toml_generation() {
        let fly = FlyIoAdapter::new("hayabusa-app")
            .region("nrt")
            .memory(512)
            .min_machines(2)
            .env("RUST_LOG", "info");
        let toml = fly.generate_fly_toml();

        assert!(toml.contains("app = \"hayabusa-app\""));
        assert!(toml.contains("primary_region = \"nrt\""));
        assert!(toml.contains("memory = \"512mb\""));
        assert!(toml.contains("min_machines_running = 2"));
        assert!(toml.contains("RUST_LOG = \"info\""));
        assert!(toml.contains("[http_service]"));
        assert!(toml.contains("force_https = true"));
    }

    #[test]
    fn test_fly_with_volume() {
        let fly = FlyIoAdapter::new("app")
            .volume("isr_cache", 1, "/data/cache");
        let toml = fly.generate_fly_toml();

        assert!(toml.contains("[[mounts]]"));
        assert!(toml.contains("source = \"isr_cache\""));
        assert!(toml.contains("destination = \"/data/cache\""));
    }

    #[test]
    fn test_fly_deploy_commands() {
        let fly = FlyIoAdapter::new("my-app")
            .add_region("lax")
            .env("SECRET", "val");
        let cmds = fly.deploy_commands();

        assert!(cmds.contains(&"fly apps create my-app".to_string()));
        assert!(cmds.contains(&"fly secrets set SECRET=val".to_string()));
        assert!(cmds.contains(&"fly deploy".to_string()));
        assert!(cmds.iter().any(|c| c.contains("lax")));
    }

    #[test]
    fn test_fly_performance_cpu() {
        let fly = FlyIoAdapter::new("app").performance_cpu(2);
        let toml = fly.generate_fly_toml();
        assert!(toml.contains("cpu_kind = \"performance\""));
        assert!(toml.contains("cpus = 2"));
    }

    // ── Docker tests ──

    #[test]
    fn test_dockerfile_generation() {
        let docker = DockerAdapter::new("my-server").port(8080);
        let df = docker.generate_dockerfile();

        assert!(df.contains("cargo-chef"));
        assert!(df.contains("cargo build --release --bin my-server"));
        assert!(df.contains("EXPOSE 8080"));
        assert!(df.contains("CMD [\"/app/my-server\"]"));
        assert!(df.contains("debian:bookworm-slim"));
    }

    #[test]
    fn test_dockerfile_without_chef() {
        let docker = DockerAdapter {
            use_cargo_chef: false,
            ..DockerAdapter::new("app")
        };
        let df = docker.generate_dockerfile();
        assert!(!df.contains("cargo-chef"));
        assert!(df.contains("cargo build --release"));
    }

    #[test]
    fn test_dockerfile_scratch() {
        let docker = DockerAdapter::new("app").scratch_runtime();
        let df = docker.generate_dockerfile();
        assert!(df.contains("FROM scratch"));
    }

    #[test]
    fn test_dockerignore() {
        let ignore = DockerAdapter::generate_dockerignore();
        assert!(ignore.contains("target/"));
        assert!(ignore.contains(".git/"));
        assert!(ignore.contains(".env"));
    }

    #[test]
    fn test_docker_compose() {
        let docker = DockerAdapter::new("app");
        let yml = docker.generate_compose(true, true);
        assert!(yml.contains("postgres:16-alpine"));
        assert!(yml.contains("redis:7-alpine"));
        assert!(yml.contains("DATABASE_URL"));
    }

    #[test]
    fn test_docker_compose_minimal() {
        let docker = DockerAdapter::new("app");
        let yml = docker.generate_compose(false, false);
        assert!(!yml.contains("postgres"));
        assert!(!yml.contains("redis"));
    }

    // ── Railway tests ──

    #[test]
    fn test_railway_config() {
        let railway = RailwayAdapter::new()
            .start_command("./target/release/my-app")
            .replicas(2);
        let config = railway.generate_config();

        assert!(config.contains("buildCommand"));
        assert!(config.contains("./target/release/my-app"));
        assert!(config.contains("numReplicas = 2"));
        assert!(config.contains("nixpacks"));
    }

    #[test]
    fn test_railway_nixpacks() {
        let railway = RailwayAdapter::new();
        let nix = railway.generate_nixpacks();
        assert!(nix.contains("rustc"));
        assert!(nix.contains("cargo"));
        assert!(nix.contains("openssl"));
    }

    // ── AWS tests ──

    #[test]
    fn test_ecs_task_definition() {
        let aws = AwsAdapter::ecs("my-app", "ap-northeast-1");
        let td = aws.generate_task_definition();
        assert!(td.contains("\"family\": \"my-app\""));
        assert!(td.contains("FARGATE"));
        assert!(td.contains("ap-northeast-1"));
    }

    #[test]
    fn test_sam_template() {
        let aws = AwsAdapter::lambda("my-func", "us-east-1");
        let sam = aws.generate_sam_template();
        assert!(sam.contains("provided.al2"));
        assert!(sam.contains("my-func"));
        assert!(sam.contains("HttpApi"));
    }

    // ── Cloudflare tests ──

    #[test]
    fn test_wrangler_toml() {
        let cf = CloudflareAdapter::new("my-worker")
            .route("example.com/*")
            .kv("CACHE", "abc123")
            .var("API_KEY", "secret");
        let toml = cf.generate_wrangler_toml();

        assert!(toml.contains("name = \"my-worker\""));
        assert!(toml.contains("route = \"example.com/*\""));
        assert!(toml.contains("binding = \"CACHE\""));
        assert!(toml.contains("API_KEY = \"secret\""));
        assert!(toml.contains("worker-build"));
    }

    #[test]
    fn test_cloudflare_minimal() {
        let cf = CloudflareAdapter::new("app");
        let toml = cf.generate_wrangler_toml();
        assert!(toml.contains("name = \"app\""));
        assert!(!toml.contains("route ="));
    }

    // ── DeployGenerator tests ──

    #[test]
    fn test_deploy_generator_creation() {
        let gen = DeployGenerator::new("/tmp/project", "my-bin").port(8080);
        assert_eq!(gen.port, 8080);
        assert_eq!(gen.binary_name, "my-bin");
    }
}
