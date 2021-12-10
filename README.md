# elm-vendor

A **WORK-IN-PROGRESS** way to vendor elm dependencies.

This tool is not yet in a working state and is only uploaded to illustrate the idea.

## Usage

elm-vendor will create an `elm-vendor.json` next to your `elm.json` so that main dependencies, source directories, and other metadata survives any modifications to `elm.json`. This will be set up with `elm-vendor init`.

To make sure this stays in sync, `elm-vendor install $dependency` should be used. This will even use either `elm install` or `lamdera install` by detecting any lamdera dependencies in your project.

Ensuring `elm-vendor.json` is in sync with `elm.json` can be done with `elm-vendor check`, which could be run during a CI / an automated build.

`elm-vendor.json` also contains a list of directories that are vendored packages. **elm-vendor deliberately does not make choices about how you download or update those packages.**

You can add to this list with `elm-vendor vendor $dir_path`, which will also update `elm.json` accordingly. `elm-vendor vendor` with no directory specified will update `elm.json` with all of the vendored dirs listed in `elm-vendor.json`.

You can reverse this with `elm-vender unvendor [$dir_path]`.

Updating `elm.json` means adding each source directory of each vendored package to the source directory list. It also means copying each vendored package's dependencies into the dependencies of the parent project.

### Where it gets tricky

Or at least, that was the original plan.

While working on this, I realized that managing indirect dependencies was going to be a drag. Finding a version of a dependency that fits each package's constraints is relatively easy, but telling `elm install` to use that specific version? Not currently possible.

My current investigations pointed me towards a few options:

0. Force all vendored packages to use the latest version of any dependency.  
Well, this fits my needs, but that might not be useful for others.
1. Integrate `elm-json`'s `elm.json` editing tools and dependency solver.  
`elm-json` is still under heavy development, and trying to hit a moving target is hard. The solver is also a fairly custom solution, which is tricky.
2. Recreate the solver from the elm toolchain.  
I actually started looking into this. But if I thought Haskell was easy to read, I wouldn't be using elm.
3. Use [the dependency solving solution from `elm-test-rs`](https://github.com/mpizenberg/pubgrub-dependency-provider-elm).  
This seems like the most viable option so far. The solver is already separated from the rest of the project, and is based on an existing rust solution for dependency reconciliation.
