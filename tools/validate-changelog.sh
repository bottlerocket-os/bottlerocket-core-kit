#!/usr/bin/env bash

# check that all `# headers` are formatted as a version, e.g. v1.2.3
# this should be sufficient to validate the CHANGELOG for our CI, provided that
# every new tag has a corresponding CHANGELOG update, as we always parse the
# CHANGELOG between two headers with a tagged version.
if ! diff <(grep -ne '^# ' CHANGELOG.md) <(grep -ne '^# v[0-9]\+\.[0-9]\+\.[0-9]\+' CHANGELOG.md); then
  echo "CHANGELOG validation FAILED! Headers must match the regex '^# v[0-9]\+\.[0-9]\+\.[0-9]\+.'"
  exit 1
fi

# check that all versions are unique
versions=$(grep -o '^# v[0-9]\+\.[0-9]\+\.[0-9]\+' CHANGELOG.md)
duplicates=$(echo "${versions}" | sort | uniq -d)
if [[ -n "${duplicates}" ]]; then
  echo "CHANGELOG validation FAILED! Duplicate versions found:"
  echo "${duplicates}"
  exit 1
fi

# check that versions are in descending order
# sort the versions and check the diff with current set of versions
sorted_versions=$(echo "${versions}" | sort -V -r)
if ! diff <(echo "${sorted_versions}") <(echo "${versions}") > /dev/null; then
  echo "CHANGELOG validation FAILED! Versions must be in descending order."
  paste <(echo "${versions}") <(echo "${sorted_versions}") | while IFS=$'\t' read -r actual expected; do
    if [[ "${actual}" != "${expected}" ]]; then
      echo "Found ${actual} but expected ${expected}"
      break
    fi
  done
  exit 1
fi

echo "CHANGELOG validation PASSED!"
