#!/usr/bin/env -S deno run

import * as TOML from "jsr:@std/toml";

const cargo_toml = await Deno.readTextFile("../Cargo.toml");
const toml = TOML.parse(cargo_toml) as {
  workspace: { package: { version: string } };
};
const [version, ...tag_split] = toml.workspace.package.version.split("-");
const tag = tag_split.join("-");

const command = new Deno.Command("bash", {
  args: [
    "-c",
    `km set var KOMODO_VERSION ${version} -y && \
      km set var KOMODO_TAG ${tag} -y && \
      km run -y action deploy-komodo`,
  ],
});

command.spawn();