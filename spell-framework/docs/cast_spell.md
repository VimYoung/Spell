It is a macro which builds upon the internal functions of spell to create and start
an event loop. This macro is responsible for the creation of necessary binding before
running the code.
`cast_spell` uses a combination of internal functions that are not to be used by
the end user. It makes the task of entering

Mention the fact that now since the platform works only because a slint window is
created strictly after a SpellInstance submits an adapter in the global. This means
that now these structs can't be used directly.

- update code examples in `entry.md`, `README.md` SpellLock docs and add example for each case.
