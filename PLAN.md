# Dev tasks, in decreasing priority

## Testing
- [ ] Harness to set up a git repo with example (elm|lamdera) project

## Init
- [x] Check for existence
- [x] Explain and ask
- [ ] Copy fields

## Vendor
TODO: is this actually a separate command? How do you add new vendored items?
- [x] make sure elm.json has been committed
- [ ] Explain and ask
- [ ] Read elm-vendor.json vendored
- [ ] Read each vendored app's json
- [ ] contextualize the source_dirs
- [ ] reconcile all vendor deps
- [ ] write modified elm.json
    - [ ] WAIT CAN YOU EVEN USE RANGED DEPS

## Check
- [ ] See if there are dependencies added that are not in elm-vendor.json
- [ ] See if any of the other fields have changed

## Install
- [ ] Explain and ask?
- [ ] Check to see if lamdera or elm
- [ ] Call above to install package
- [ ] Extract package info to elm-vendor.json

## Unvendor
- [ ] Explain and ask (destructive to elm.json)
- [ ] recreate it without vendored deps in elm-vendor.json
