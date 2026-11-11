# measurements (print-type-sizes, debug profile; layout is the same in release)
main 89452d2ac -> PR
jsx::Pragma 152 -> 64
resolver::Result 288 -> 192   (JsxConfig 56 in place of a Pragma)
TSConfigJSON 312 -> 232
resolver::BundleOptions 960 -> 808
api::TransformOptions 584 -> 752  (jsx::Sources 224 in place of Option<api::Jsx> 56)
jsx::Stated 72, jsx::ImportSource 48 -> 8, MemberList 24

# allocations (counting allocator in a unit test of bun_options_types)
per Pragma clone, states defaults / importSource / factory / both: main 0 / 4 / 2 / 6 -> PR 0 / 0 / 0 / 0
PR per resolve of two layers: 0 in all four; per masked lay of a tsconfig: 0 in all four

# matrices (debug build of the PR vs canary 1.4.4-canary.1+89452d2ac), stage b
main matrix 4662 cells: 2673 equal to main; moved: 1235 by the rule, 86 rule + R1, 108 rule + R3, 80 R1, 7 R1+R3, 6 R2, 6 R2+R3, 461 R3; 0 unexplained
special matrix 405 cells: 197 equal; 169 by the rule, 9 rule + R1, 30 R3 (scanImports); 0 unexplained
fail-before on the canary: jsx-tsconfig-react-jsx.test.ts 13 of 24 fail; bundler_jsx.test.ts 1 new test fails; parallel.test.ts 2 new tests fail
suites on the PR debug build: all pass, except parallel.test.ts "each worker has a unique JEST_WORKER_ID" (timing under the debug build, also fails on main's debug build)

# release binary size (local `bun run build:release`, same toolchain and flags)
main 89452d2ac: build/release/bun 143,284,496 bytes; size text 143,094,946; .text 112,978,537; .rodata 27,239,404; .bss 1,848,360
PR 836e79bb4:   build/release/bun 143,259,920 bytes; size text 143,068,066; .text 112,951,657; .rodata 27,239,404; .bss 1,848,616
delta: file -24,576; .text -26,880; .bss +256
tools missing in the container: perf, valgrind, bloaty (no instruction counts)

# wall clock (interleaved A/B, 15 rounds each, `bun build` of 3,001 .tsx files beside tsconfig jsx=react-jsx + jsxImportSource + jsxFactory; local release builds)
main: wall median 1346 ms (1212 to 1744), cpu median 920 ms
PR:   wall median 1377 ms (1193 to 2538), cpu median 902 ms
the delta is below the noise floor; the output of both builds is byte-identical
