variable "EXTENSION_NAME" {
  default = "pg_row_hashes"
}

variable "REGISTRY" {
  default = "ghcr.io"
}

variable "PG_VERSION" {
  default = "18"
}

variable "EXTENSION_VERSION" {
  default = ""
}

variable "DISTROS" {
  default = ["bookworm", "trixie"]
}

variable "DISTRO" {
  default = "bookworm"
}

variable "BRANCH_NAME" {
  default = ""
}

target "extension" {
  # dockerfile = "../Dockerfile"
  tags = [
    "${REGISTRY}/${EXTENSION_NAME}:${PG_VERSION}-${EXTENSION_VERSION}-${formatdate("YYYYMMDDHHMM", timestamp())}-${DISTRO}",
    "${REGISTRY}/${EXTENSION_NAME}:${PG_VERSION}-${EXTENSION_VERSION}-${DISTRO}"
  ]
  args = {
    PG_VERSION = PG_VERSION
    DISTRO = DISTRO
  }
}

target "extension-feature" {
  # dockerfile = "../Dockerfile"
  tags = [
    "${REGISTRY}/${EXTENSION_NAME}:${PG_VERSION}-${EXTENSION_VERSION}-${BRANCH_NAME}-${DISTRO}"
  ]
  args = {
    PG_VERSION = PG_VERSION
    DISTRO = DISTRO
  }
}

# Matrix builds for PG 18 with multiple distros
group "extension-all" {
  targets = [
    "extension-18-bookworm",
    "extension-18-trixie"
  ]
}

target "extension-18-bookworm" {
  inherits = ["extension"]
  args = {
    PG_VERSION = "18"
    DISTRO = "bookworm"
  }
}

target "extension-18-trixie" {
  inherits = ["extension"]
  args = {
    PG_VERSION = "18"
    DISTRO = "trixie"
  }
}
