TOP := $(dir $(abspath $(firstword $(MAKEFILE_LIST))))
TOOLS_DIR := $(TOP)tools
TWOLITER_DIR := $(TOOLS_DIR)/twoliter
TWOLITER := $(TWOLITER_DIR)/twoliter
CARGO_HOME := $(TOP).cargo

TWOLITER_VERSION ?= "0.18.0"
TWOLITER_SHA256_AARCH64 ?= "48949bc0b650eb6aabe9e60baed207df2700e3e2cf17006af1ee581b89df71e9"
TWOLITER_SHA256_X86_64 ?= "8591aa68cfc145578bb0ce4e22a2c3108fc6fe318a9793d0f1cb764d67b28834"
KIT ?= bottlerocket-core-kit
UNAME_ARCH = $(shell uname -m)
ARCH ?= $(UNAME_ARCH)
VENDOR ?= bottlerocket
UPSTREAM_SOURCE_FALLBACK ?= false

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
ifeq ($(UPSTREAM_SOURCE_FALLBACK), false)
	@$(TWOLITER) build kit $(KIT) --arch $(ARCH)
else
	@$(TWOLITER) build kit $(KIT) --arch $(ARCH) --upstream-source-fallback
endif

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
