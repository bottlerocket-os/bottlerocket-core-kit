TOP := $(dir $(abspath $(firstword $(MAKEFILE_LIST))))
TOOLS_DIR := $(TOP)tools
TWOLITER_DIR := $(TOOLS_DIR)/twoliter
TWOLITER := $(TWOLITER_DIR)/twoliter
CARGO_HOME := $(TOP).cargo

TWOLITER_VERSION ?= "0.5.0"
TWOLITER_SHA256_AARCH64 ?= "cec8d30377f5cb38ee1d3bc99bb8aaf3958213b38be6a75d09a8bc5fcd3da590"
TWOLITER_SHA256_X86_64 ?= "d580180969f8b34b1af5d2524ff024e90432f09f991fc044444019da20a027a8"
KIT ?= bottlerocket-core-kit
UNAME_ARCH = $(shell uname -m)
ARCH ?= $(UNAME_ARCH)
VENDOR ?= bottlerocket

ifeq ($(UNAME_ARCH), aarch64)
	TWOLITER_SHA256=$(TWOLITER_SHA256_AARCH64)
else
	TWOLITER_SHA256=$(TWOLITER_SHA256_X86_64)
endif


export GO_MODULES = ecs-gpu-init host-ctr

all: build

prep:
	@mkdir -p $(TWOLITER_DIR)
	@mkdir -p $(CARGO_HOME)
	@$(TOOLS_DIR)/install-twoliter.sh \
		--repo "https://github.com/bottlerocket-os/twoliter" \
		--version v$(TWOLITER_VERSION) \
		--directory $(TWOLITER_DIR) \
		--reuse-existing-install \
		--allow-binary-install $(TWOLITER_SHA256) \
		--allow-from-source

update: prep
	@$(TWOLITER) update

fetch: prep
	@$(TWOLITER) fetch --arch $(ARCH)

build: fetch 
	@$(TWOLITER) build kit $(KIT) --arch $(ARCH)

publish: prep
	@$(TWOLITER) publish kit $(KIT) $(VENDOR)

TWOLITER_MAKE = $(TWOLITER) make --cargo-home $(CARGO_HOME) --arch $(ARCH)

# Treat any targets after "make twoliter" as arguments to "twoliter make".
ifeq (twoliter,$(firstword $(MAKECMDGOALS)))
  TWOLITER_MAKE_ARGS := $(wordlist 2,$(words $(MAKECMDGOALS)),$(MAKECMDGOALS))
  $(eval $(TWOLITER_MAKE_ARGS):;@:)
endif

# Transform "make twoliter" into "twoliter make", for access to tasks that are
# only available through the embedded Makefile.toml.
twoliter: prep
	@$(TWOLITER_MAKE) $(TWOLITER_MAKE_ARGS)

.PHONY: prep update fetch build publish twoliter
