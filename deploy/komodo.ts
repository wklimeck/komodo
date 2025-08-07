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

// Cargo check first here to make sure lock file is updated before commit.
const cmd = `
cargo check
echo ""

git add --all
git commit --all --message "deploy ${version}-${tag}-${next_count}"

echo ""
git push
echo ""

km set var KOMODO_VERSION ${version} -y
echo ""

km set var KOMODO_TAG ${tag}-${next_count} -y
echo ""

km run -y action deploy-komodo
`
  .split("\n")
  .map((line) => line.trim())
  .filter((line) => line.length > 0 && !line.startsWith("//"))
  .join(" && ");

new Deno.Command("bash", {
  args: ["-c", cmd],
}).spawn();
