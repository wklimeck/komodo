import * as TOML from "jsr:@std/toml";

const cargo_toml_str = await Deno.readTextFile("Cargo.toml");
const prev_version = (
  TOML.parse(cargo_toml_str) as {
    workspace: { package: { version: string } };
  }
).workspace.package.version;

const [version, tag, count] = prev_version.split("-");
const next_count = Number(count) + 1;

const next_version = `${version}-${tag}-${next_count}`;

await Deno.writeTextFile(
  "Cargo.toml",
  cargo_toml_str.replace(
    `version = "${prev_version}"`,
    `version = "${next_version}"`
  )
);

const command = new Deno.Command("bash", {
  args: [
    "-c",
    // Cargo check here to make sure lock file is updated before commit.
    `cargo check && echo "" && \
      git add --all && \
      git commit --all --message "deploy ${version}-${tag}-${next_count}" && \
      git push && echo "" \
      km set var KOMODO_VERSION ${version} -y && \
      km set var KOMODO_TAG ${tag}-${next_count} -y && \
      km run -y action deploy-komodo`,
  ],
});

command.spawn();
