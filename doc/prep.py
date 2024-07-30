from pathlib import Path
import re


ROOT = Path(__file__).parent.absolute()


def change_readme_links(path_from: Path, path_to: Path):
    with path_from.open() as f:
        content = f.read()
    content, _ = re.subn(r"\[`(.+)`\]\(https://ducer.readthedocs.io.+#\1\)", r"[](\1)", content)
    content, _ = re.subn(r"\[(.+)\]\(https://ducer.readthedocs.io.+#(.+)\)", r"[\1](\2)", content)
    with path_to.open("wt", encoding="utf-8") as f:
        f.write(content)


if __name__ == "__main__":
    change_readme_links(
        ROOT.parent / "README.md",
        ROOT / "source" / "README.md",
    )
