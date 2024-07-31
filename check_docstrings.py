import ast
import re
import importlib
import inspect
from dataclasses import dataclass


PATTERN_DIRECTIVE = re.compile(
    r"^(?P<indentation> *)\.\. (?P<type>\w+):: (?P<name>\w+)(?P<signature>\(.+\)(?: -> .+)?)?",
    flags=re.MULTILINE,
)
PATTERN_SIGNATURE = re.compile(r"^(?P<indentation> *)def (?P<name>\w+)(?P<signature>\(.+\)(?: -> .+)?):")
PATTERN_RST_PREFIX = re.compile(r":\w+:`")
PATTERN_SIMPLIFY_SIGNATURE = re.compile(r"~(\w+\.)+")
PATTERN_SIMPLIFY_DOCSTRING = re.compile(r"[\W\s]")


@dataclass
class Item:
    indentation: int | None
    type: str | None
    name: str
    signature: str | None
    start: int | None
    doc_start: int | None
    doc_end: int | None
    docstring: str | None


def get_reference_items():
    with open("doc/source/api_reference.rst") as f:
        reference = f.read()

    items = []
    for match in PATTERN_DIRECTIVE.finditer(reference):
        items.append(Item(
            indentation=len(match.group("indentation")),
            type=match.group("type"),
            name=match.group("name"),
            signature=match.group("signature"),
            start=match.start(),
            doc_start=match.end(),
            doc_end=None,
            docstring=None,
        ))

    # use start of next item as doc_end
    for i1, i2 in zip(items, items[1:]):
        i1.doc_end = i2.start
    # last item has docstring until end of text
    items[-1].doc_end = len(reference)

    # add docstrings, fix names of methods/attributes
    current_class = None
    for item in  items:
        # TODO should check minimum indentation level in docstring
        item.docstring = reference[item.doc_start:item.doc_end]
        if item.type == "class":
            current_class = item.name
        elif item.type in ("attribute", "method", "classmethod"):
            if current_class is None:
                raise SyntaxError(f"{item} must be part of a class")
            item.name = f"{current_class}.{item.name}"
        else:
            current_class = None

    return {item.name: item for item in items}


def make_stub_item(prefix, node):
    signature = None
    try:
        code = ast.unparse(node)
        match = PATTERN_SIGNATURE.search(code)
        signature = match.group("signature")
    except AttributeError:
        pass
    docstring = None
    try:
        docstring = ast.get_docstring(node)
    except TypeError:
        docstring = None
    return Item(
        indentation=None,
        type=None,
        name=prefix + node.name,
        signature=signature,
        start=None,
        doc_start=None,
        doc_end=None,
        docstring=docstring,
    )


def get_stub_items():
    with open("ducer/_fst.pyi", 'r') as f:
        pyi_content = f.read()
    items = []
    tree = ast.parse(pyi_content)
    todo = [("", child) for child in ast.iter_child_nodes(tree)]
    while todo:
        prefix, node = todo.pop()
        if not hasattr(node, "name"):
            continue
        try:
            items.append(make_stub_item(prefix, node))
        except TypeError:
            continue
        prefix += node.name + "."
        if isinstance(node, (ast.ClassDef)):
            todo.extend((prefix, child) for child in ast.iter_child_nodes(node))
    return {item.name: item for item in items}


def make_module_item(prefix, obj):
    signature = None
    try:
        signature = str(inspect.signature(obj))
    except ValueError:
        pass
    return Item(
        indentation=None,
        type=None,
        name=prefix + obj.__name__,
        signature=signature,
        start=None,
        doc_start=None,
        doc_end=None,
        docstring=inspect.getdoc(obj),
    )


def get_module_items():
    module = importlib.import_module("ducer._fst")
    items = []
    for top_name, obj in inspect.getmembers(module):
        try:
            items.append(make_module_item("", obj))
        except TypeError:
            pass
        if inspect.isclass(obj):
            prefix = top_name + "."
            for _, obj in inspect.getmembers(obj):
                try:
                    items.append(make_module_item(prefix, obj))
                except TypeError:
                    pass
    return {item.name: item for item in items}


def simplify_docstring(docstring):
    docstring, _ = PATTERN_RST_PREFIX.subn("", docstring)
    docstring, _ = PATTERN_SIMPLIFY_DOCSTRING.subn("", docstring)
    docstring = docstring.lower()
    return docstring


def docstrings_differ(item1, item2):
    return simplify_docstring(item1.docstring) != simplify_docstring(item2.docstring)


def simplify_signature(signature):
    if signature is None:
        return None
    signature, _ = PATTERN_SIMPLIFY_SIGNATURE.subn("", signature)
    return signature


def signatures_differ(item1, item2):
    return simplify_signature(item1.signature) != simplify_signature(item2.signature)


def main():
    reference = get_reference_items()
    stub = get_stub_items()
    module = get_module_items()

    for name, item in reference.items():
        if name not in stub:
            print()
            print()
            print("#======================================")
            print(f"#{name} missing in stub!")
            continue
        other = stub[name]
        if docstrings_differ(item, other):
            print()
            print("#======================================")
            print(f"#{name}: reference and stub docstrings differ")
            print()
            print("#reference")
            print(item.docstring)
            print()
            print("#stub")
            print(other.docstring)
        if signatures_differ(item, other):
            print()
            print()
            print("======================================")
            print(f"{name}: reference and stub signatures differ")
            print()
            print("reference")
            print(item.signature)
            print()
            print("stub")
            print(other.signature)

    pass


if __name__ == "__main__":
    main()
