import inspect
import ducer


def include(name, exclude=("__class__",)):
    return name not in exclude and (name.startswith('__') or not name.startswith('_'))


def print_docstrings(obj, prefix=''):
    """Recursively print the docstrings of all public members of the given object."""
    if inspect.ismodule(obj):
        # Iterate over all attributes in the module
        for name, sub_obj in inspect.getmembers(obj):
            if include(name):
                print_docstrings(sub_obj, f"{prefix}{name}.")
    elif inspect.isclass(obj):
        # Print class docstring
        docstring = inspect.getdoc(obj)
        print(f"\nClass: {prefix}{obj.__name__}")
        print(f"{'=' * (len(prefix) + len(obj.__name__) + 7)}")
        if docstring:
            print(docstring)
        else:
            print("No docstring")

        # Iterate over all members of the class
        for name, sub_obj in inspect.getmembers(obj):
            if include(name):
                print_docstrings(sub_obj, f"{prefix}{obj.__name__}.")
    elif inspect.isfunction(obj):
        # Print function docstring
        print(f"\nFunction: {prefix}{obj.__name__}")
        print(f"{'=' * (len(prefix) + len(obj.__name__) + 9)}")
        docstring = inspect.getdoc(obj)
        if docstring:
            print(docstring)
        else:
            print("No docstring")
    elif inspect.isroutine(obj):
        # Print method docstring
        print(f"\nMethod: {prefix}{obj.__name__}")
        print(f"{'=' * (len(prefix) + len(obj.__name__) + 8)}")
        docstring = inspect.getdoc(obj)
        if docstring:
            print(docstring)
        else:
            print("No docstring")

if __name__ == "__main__":
    print_docstrings(ducer)
