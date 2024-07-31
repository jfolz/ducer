import importlib
import inspect
import ast
import sys


def get_docstrings(module):
    docstrings = {}
    for name, obj in inspect.getmembers(module):
        if inspect.isfunction(obj) or inspect.isclass(obj):
            docstrings[name] = inspect.getdoc(obj)
            if inspect.isclass(obj):
                for method_name, method in inspect.getmembers(obj):
                    docstrings[f"{name}.{method_name}"] = inspect.getdoc(method)
    return docstrings


def compare_docstrings(pyi_path, docstrings):
    ret = 0
    with open(pyi_path, 'r') as f:
        pyi_content = f.read()
    tree = ast.parse(pyi_content)
    todo = [("", child) for child in ast.iter_child_nodes(tree)]
    while todo:
        path, node = todo.pop()
        try:
            name = node.name
            path += name
        except AttributeError:
            name = ""
            pass
        try:
            docstring = ast.get_docstring(node)
        except TypeError:
            docstring = None
        if docstring is not None and path in docstrings:
            if docstring.strip() != docstrings[path].strip():
                print(f"{path} docstring differs")
                print(docstring.strip())
                print(docstrings[path].strip())
                ret = 1
        elif path in docstrings and docstring is None:
            if not (name.startswith("__") and name.endswith("__")):
                print(f"{path} docstring missing")
                ret = 1
        # TODO string replace old docstring with new docstring
        if path:
            path += "."
        if isinstance(node, (ast.ClassDef)):
            todo.extend((path, child) for child in ast.iter_child_nodes(node))
    return ret


def main():
    module = importlib.import_module("ducer._fst")
    docstrings = get_docstrings(module)
    sys.exit(compare_docstrings("ducer/_fst.pyi", docstrings))


if __name__ == '__main__':
    main()
