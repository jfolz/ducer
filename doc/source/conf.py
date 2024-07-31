# Configuration file for the Sphinx documentation builder.
#
# For the full list of built-in configuration values, see the documentation:
# https://www.sphinx-doc.org/en/master/usage/configuration.html

import os
import re
import sys
sys.path.insert(0, os.path.abspath("../.."))

# -- Project information -----------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#project-information

project = "ducer"
copyright = "2024, Joachim Folz"
author = "Joachim Folz"

# -- General configuration ---------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#general-configuration

extensions = [
    "sphinx.ext.extlinks",
    "sphinx.ext.intersphinx",
    "sphinx.ext.napoleon",
    "myst_parser",
]

templates_path = ["_templates"]
exclude_patterns = []

myst_ref_domains = ["py", "python"]

# -- Options for HTML output -------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#options-for-html-output

html_title = project
html_theme = "furo"
html_static_path = ["_static"]
html_theme_options = {
    "light_logo": "img/duck.svg",
    "dark_logo": "img/duck.svg",
    "sidebar_hide_name": True,
    "light_css_variables": {
        "color-brand-primary": "#005fc8",
        "color-brand-content": "#005fc8",
        "color-api-pre-name": "#008151",
        "color-api-name": "#008151",
        "color-highlight-on-target": "rgba(101, 31, 255, 0.2)",
        "color-background-hover--transparent": "#80808010",
    },
    "dark_css_variables": {
        "color-brand-primary": "#00b9ec",
        "color-brand-content": "#00b9ec",
        "color-api-pre-name": "#00bfa5",
        "color-api-name": "#00bfa5",
        "color-highlight-on-target": "rgba(101, 31, 255, 0.2)",
        "color-background-hover--transparent": "#80808010",
    },
    "source_repository": "https://github.com/jfolz/ducer",
    "source_branch": "main",
    "source_directory": "doc/source/",
}
html_sidebars = {
    "**": [
        "sidebar/brand.html",
        "sidebar/search.html",
        "sidebar/scroll-start.html",
        "sidebar/navigation.html",
        "links.html",
        "sidebar/ethical-ads.html",
        "sidebar/scroll-end.html",
        "sidebar/variant-selector.html",
    ]
}
html_css_files = [
    "css/custom.css",
]

# -- Options for intersphinx extension ---------------------------------------
# https://www.sphinx-doc.org/en/master/usage/extensions/intersphinx.html#configuration

intersphinx_mapping = {
    "python": ("https://docs.python.org/3", None),
    "msgpack": ("https://msgpack-python.readthedocs.io/en/latest/", None),
}
