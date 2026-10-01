#!/bin/bash

# STUFFIE Stress Test
# Tests:
#   1. Cargo compilation
#   2. FullAuto parent + SemiAuto child
#   3. SemiAuto parent + SemiAuto child
#   4. SingleBreak parent + SemiAuto child
#   5. Output/log directory creation
#   6. Repeated execution
#   7. Missing configuration
#   8. Missing child configuration
#   9. Circular pipeline dependency

set -u

PASS=0
FAIL=0

TEST_OUTPUT="./test_output"
TEMP_DIR="./stress_test_tmp"

print_test() {
    echo
    echo "========================================"
    echo "$1"
    echo "========================================"
}

pass_test() {
    echo "PASS: $1"
    PASS=$((PASS + 1))
}

fail_test() {
    echo "FAIL: $1"
    FAIL=$((FAIL + 1))
}

# --------------------------------------------------
# Cleanup
# --------------------------------------------------

rm -rf "$TEST_OUTPUT"
rm -rf "$TEMP_DIR"

mkdir -p "$TEMP_DIR"

# --------------------------------------------------
# 1. Cargo check
# --------------------------------------------------

print_test "1. Cargo check"

if cargo check; then
    pass_test "cargo check"
else
    fail_test "cargo check"
fi

# --------------------------------------------------
# 2. FullAuto parent + SemiAuto child
# --------------------------------------------------

print_test "2. FullAuto parent + SemiAuto child"

rm -rf "$TEST_OUTPUT"

OUTPUT=$(printf '\n' | cargo run -- -a -c parent.toml 2>&1)
STATUS=$?

echo "$OUTPUT"

if [ "$STATUS" -eq 0 ] &&
   echo "$OUTPUT" | grep -q "Starting pipeline: parent_pipeline" &&
   echo "$OUTPUT" | grep -q "Starting pipeline: child_pipeline" &&
   echo "$OUTPUT" | grep -q "Running stage: Child First" &&
   echo "$OUTPUT" | grep -q "Running stage: Child Second" &&
   echo "$OUTPUT" | grep -q "Running stage: Parent Final"
then
    pass_test "FullAuto parent + SemiAuto child"
else
    fail_test "FullAuto parent + SemiAuto child"
fi

# --------------------------------------------------
# 3. SemiAuto parent + SemiAuto child
# --------------------------------------------------

print_test "3. SemiAuto parent + SemiAuto child"

rm -rf "$TEST_OUTPUT"

OUTPUT=$(printf '\n' | cargo run -- -s -c parent.toml 2>&1)
STATUS=$?

echo "$OUTPUT"

if [ "$STATUS" -eq 0 ] &&
   echo "$OUTPUT" | grep -q "Running stage: Parent First" &&
   echo "$OUTPUT" | grep -q "Running stage: Child First" &&
   echo "$OUTPUT" | grep -q "Running stage: Child Second" &&
   echo "$OUTPUT" | grep -q "Running stage: Parent Final"
then
    pass_test "SemiAuto parent + SemiAuto child"
else
    fail_test "SemiAuto parent + SemiAuto child"
fi

# --------------------------------------------------
# 4. SingleBreak parent + SemiAuto child
# --------------------------------------------------

print_test "4. SingleBreak parent + SemiAuto child"

rm -rf "$TEST_OUTPUT"

# Three Enter presses:
#   Parent First
#   Child First
#   Parent Final
OUTPUT=$(printf '\n\n\n' | cargo run -- -b -c parent.toml 2>&1)
STATUS=$?

echo "$OUTPUT"

PAUSES=$(echo "$OUTPUT" | grep -c "Press Enter to continue..." || true)

if [ "$STATUS" -eq 0 ] &&
   [ "$PAUSES" -eq 3 ] &&
   echo "$OUTPUT" | grep -q "Running stage: Parent First" &&
   echo "$OUTPUT" | grep -q "Running stage: Child First" &&
   echo "$OUTPUT" | grep -q "Running stage: Child Second" &&
   echo "$OUTPUT" | grep -q "Running stage: Parent Final"
then
    pass_test "SingleBreak parent + SemiAuto child"
else
    fail_test "SingleBreak parent + SemiAuto child"
fi

# --------------------------------------------------
# 5. Output and log directory creation
# --------------------------------------------------

print_test "5. Output and log directory creation"

if [ -d "$TEST_OUTPUT/parent/logs" ] &&
   [ -d "$TEST_OUTPUT/child/logs" ] &&
   [ -d "$TEST_OUTPUT/child/.stuffie" ] &&
   [ -f "$TEST_OUTPUT/child/.stuffie/resolved_config.json" ]
then
    pass_test "Output, log, and resolved-config directories/files"
else
    fail_test "Output, log, and resolved-config directories/files"
fi

# --------------------------------------------------
# 6. Log files
# --------------------------------------------------

print_test "6. Log file creation"

if [ -f "$TEST_OUTPUT/parent/logs/Parent_First.log" ] &&
   [ -f "$TEST_OUTPUT/parent/logs/Parent_Final.log" ] &&
   [ -f "$TEST_OUTPUT/child/logs/Child_First.log" ] &&
   [ -f "$TEST_OUTPUT/child/logs/Child_Second.log" ]
then
    pass_test "Expected log files created"
else
    fail_test "Expected log files created"
fi

# --------------------------------------------------
# 7. Repeated execution
# --------------------------------------------------

print_test "7. Repeated execution"

OUTPUT=$(printf '\n' | cargo run -- -a -c parent.toml 2>&1)
STATUS=$?

if [ "$STATUS" -eq 0 ]; then
    pass_test "Pipeline can run again with existing output"
else
    echo "$OUTPUT"
    fail_test "Pipeline can run again with existing output"
fi

# --------------------------------------------------
# 8. Missing configuration
# --------------------------------------------------

print_test "8. Missing configuration"

if cargo run -- -a -c does_not_exist.toml > /dev/null 2>&1; then
    fail_test "Missing configuration rejected"
else
    pass_test "Missing configuration rejected"
fi

# --------------------------------------------------
# 9. Missing child configuration
# --------------------------------------------------

print_test "9. Missing child configuration"

cat > "$TEMP_DIR/missing_child_parent.toml" <<EOF
[pipeline]
name = "missing_child_parent"
output_dir = "./test_output/missing_child"
source_dir = "./test_source"
mode = "full_auto"

[[stages]]
name = "Missing Child"
type = "pipeline"
path = "does_not_exist.toml"
EOF

if cargo run -- -a -c "$TEMP_DIR/missing_child_parent.toml" > /dev/null 2>&1; then
    fail_test "Missing child configuration rejected"
else
    pass_test "Missing child configuration rejected"
fi

# --------------------------------------------------
# 10. Circular dependency
# --------------------------------------------------

print_test "10. Circular pipeline dependency"

cat > "$TEMP_DIR/circular_a.toml" <<EOF
[pipeline]
name = "circular_a"
output_dir = "./test_output/circular_a"
source_dir = "./test_source"
mode = "full_auto"

[[stages]]
name = "Pipeline B"
type = "pipeline"
path = "circular_b.toml"
EOF

cat > "$TEMP_DIR/circular_b.toml" <<EOF
[pipeline]
name = "circular_b"
output_dir = "./test_output/circular_b"
source_dir = "./test_source"
mode = "full_auto"

[[stages]]
name = "Pipeline A"
type = "pipeline"
path = "circular_a.toml"
EOF

if cargo run -- -a -c "$TEMP_DIR/circular_a.toml" > /tmp/stuffie_circular_test.log 2>&1; then
    fail_test "Circular dependency rejected"
else
    if grep -q "Circular dependency detected" /tmp/stuffie_circular_test.log; then
        pass_test "Circular dependency rejected"
    else
        echo "Expected circular-dependency message:"
        cat /tmp/stuffie_circular_test.log
        fail_test "Circular dependency rejected"
    fi
fi

# --------------------------------------------------
# Summary
# --------------------------------------------------

echo
echo "========================================"
echo "STUFFIE STRESS TEST SUMMARY"
echo "========================================"
echo "Passed: $PASS"
echo "Failed: $FAIL"
echo "========================================"

rm -rf "$TEMP_DIR"

if [ "$FAIL" -eq 0 ]; then
    echo "ALL AUTOMATED TESTS PASSED"
    exit 0
else
    echo "SOME TESTS FAILED"
    exit 1
fi