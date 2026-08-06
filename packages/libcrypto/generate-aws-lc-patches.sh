#!/bin/bash

git clone https://github.com/aws/aws-lc
cd aws-lc
git checkout origin/fips-2024-09-27
git format-patch --start-number=1001 --no-numbered --no-signature \
    AWS-LC-FIPS-3.3.0..
