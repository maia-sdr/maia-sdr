# Contributing to Maia SDR

## DCO

Contributions submitted for inclussion in Maia SDR shall be licensed under the
same licenses of the components affected by the contribution. It is therefore
imperative that code submissions belong to the authors, and that submitters have
the authority to merge that code into the public Maia SDR codebase.

For that purpose, we use the [Developer's Certificate of Origin](DCO.txt). It is
the same document used by other projects. Signing the DCO states that there are
no legal reasons to not merge your code.

To sign the DCO, suffix your git commits with a "Signed-off-by" line. When using
the command line, you can use `git commit -s` to automatically add this line. If
there were multiple authors of the code, or other types of stakeholders, make
sure that all are listed, each with a separate Signed-off-by line.

## Coding guidelines

The usual coding guidelines apply for each language. For Python we follow PEP8,
and for Rust we use `rustfmt` to format the code.

## Git commit messages

We follow standard git commit message guidelines, similar to many other open
source projects. See the coding guidelines for more details. In a nutshell:

* Keep the lines below 72 characters.
* Subject line has the component prepended (e.g., `maia-hdl:`).
* Avoid empty git commit messages.
* The git commit message explains the change, the code only explains the current
  state.
