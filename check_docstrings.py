import ast
import re
import importlib
import inspect
from dataclasses import dataclass


PATTERN_DIRECTIVE = re.compile(
    r"^(?P<indentation> *)\.\. (?P<type>\w+):: (?P<name>\w+)(?P<signature>\(.+\)(?: -> .+)?)?",
    flags=re.MULTILINE,
)
PATTERN_SIGNATURE = re.compile(
    r"^(?P<indentation> *)def (?P<name>\w+)(?P<signature>\(.+\)(?: -> .+)?):",
    flags=re.MULTILINE,
)
PATTERN_RST_PREFIX = re.compile(r":\w+:`")
PATTERN_SHORT_REFERENCE = re.compile(r"~(\w+\.)+")
PATTERN_SIMPLIFY_DOCSTRING = re.compile(r"[\W\s]")
PATTERN_MAGIC_METHOD = re.compile(r"\w+\.__[a-zA-Z0-9]+__")


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


def make_stub_item_assign(prefix, node, docstring):
    return Item(
        indentation=None,
        type=None,
        name=prefix + node.targets[0].id,
        signature=None,
        start=None,
        doc_start=None,
        doc_end=None,
        docstring=docstring,
    )


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
    previous_value = None
    while todo:
        prefix, node = todo.pop()
        if prefix == "Op.":
            pass
        # remember orphaned values
        if isinstance(node, ast.Expr):
            previous_value = node.value.value
        # handle class variables
        if isinstance(node, ast.Assign):
            items.append(make_stub_item_assign(prefix, node, previous_value))
            previous_value = None
            continue
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
    docstring, _ = PATTERN_SHORT_REFERENCE.subn("", docstring)
    docstring, _ = PATTERN_RST_PREFIX.subn("", docstring)
    docstring, _ = PATTERN_SIMPLIFY_DOCSTRING.subn("", docstring)
    docstring = docstring.lower()
    return docstring


def docstrings_differ(item1, item2):
    d1 = item1.docstring
    d2 = item2.docstring
    if d1 is None or d2 is None:
        return d1 != d2
    d1 = simplify_docstring(d1)
    d2 = simplify_docstring(d2)
    return d1 != d2


def simplify_signature(signature):
    if signature is None:
        return None
    signature, _ = PATTERN_SHORT_REFERENCE.subn("", signature)
    return signature


def signatures_differ(item1, item2):
    s1 = simplify_signature(item1.signature)
    s2 = simplify_signature(item2.signature)
    return s1 != s2


def is_magic(name):
    return PATTERN_MAGIC_METHOD.search(name) is not None


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
            print()
            print("#======================================")
            print(f"#{name}: reference and stub docstrings differ")
            print()
            print("#reference")
            print(item.docstring)
            print()
            print("#stub")
            print(other.docstring)
        # TODO fix signature comparison for classes
        # e.g., reference is `(data: SupportsBytes)`,
        #            stub is `(self, data: SupportsBytes)`
        if item.type != "class" and signatures_differ(item, other):
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

    for name, item in stub.items():
        # workaround: don't check magic methods, Rust docstrings are not considered
        if is_magic(name):
            continue
        if name not in module:
            # workaround: attributes can't have docstrings
            if not name.startswith("Op."):
                print()
                print()
                print("#======================================")
                print(f"#{name} missing in module!")
            continue
        other = module[name]
        if docstrings_differ(item, other):
            print()
            print()
            print("#======================================")
            print(f"#{name}: stub and module docstrings differ")
            print()
            print("#stub")
            print(item.docstring)
            print()
            print("#module")
            print(other.docstring)


if __name__ == "__main__":
    main()
