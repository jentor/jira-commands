"""Validate the portable Jira package and its client catalogs offline."""

import json
import re
from pathlib import Path

import jsonschema
import yaml


ROOT = Path(__file__).resolve().parents[2]


def load(path):
    return json.loads((ROOT / path).read_text(encoding="utf-8"))


def validate():
    assert not (ROOT / "plugin/mcp.json").exists(), "MCP must remain opt-in, outside the auto-discovered root"
    for component, path in (("plugin", "plugin/plugin.json"), ("mcp", "plugin/examples/mcp.json")):
        schema = load(f".github/schemas/agent-plugins/1.0.0/{component}.schema.json")
        jsonschema.Draft202012Validator.check_schema(schema)
        jsonschema.validate(load(path), schema)

    manifest = load("plugin/plugin.json")
    claude = load("plugin/.claude-plugin/plugin.json")
    marketplace = load(".claude-plugin/marketplace.json")
    assert manifest["name"] == claude["name"] == marketplace["plugins"][0]["name"]
    version = (ROOT / "plugin/VERSION").read_text().strip()
    assert all(value == version for value in (
        manifest["version"], claude["version"], marketplace["metadata"]["version"],
        marketplace["plugins"][0]["version"],
    )), "Plugin versions differ"
    source = marketplace["plugins"][0]["source"]
    assert source["source"] == "git-subdir" and source["path"] == "plugin"

    catalog = load(".agents/plugins/marketplace.json")
    entry = catalog["plugins"][0]
    assert catalog["name"] == "jira-commands" and entry["name"] == manifest["name"]
    assert entry["policy"]["installation"] == "AVAILABLE"
    source = entry["source"]
    assert source["source"] == "local" and source["path"].startswith("./")
    assert (ROOT / source["path"]).resolve() == (ROOT / "plugin").resolve()

    skills = sorted((ROOT / "plugin/skills").glob("*/SKILL.md"))
    assert skills, "No skills found"
    for path in skills:
        text = path.read_text(encoding="utf-8")
        assert text.startswith("---\n"), f"Missing frontmatter: {path}"
        parts = text.split("---", 2)
        assert len(parts) == 3 and parts[2].strip(), f"Missing skill body: {path}"
        metadata = yaml.safe_load(parts[1])
        assert isinstance(metadata, dict), f"Invalid frontmatter: {path}"
        name = metadata.get("name")
        assert name == path.parent.name and len(name) <= 64, f"Invalid name: {path}"
        assert re.fullmatch(r"[a-z0-9]+(?:-[a-z0-9]+)*", name), f"Invalid name: {path}"
        description = metadata.get("description")
        assert isinstance(description, str) and 0 < len(description.strip()) <= 1024, (
            f"Invalid description: {path}"
        )
        assert set(metadata) <= {
            "name", "description", "license", "compatibility", "metadata", "allowed-tools",
        }, f"Unknown frontmatter: {path}"
    print(f"Validated Agent Plugins 1.0.0 package, {len(skills)} skills, and client catalogs")


if __name__ == "__main__":
    validate()
