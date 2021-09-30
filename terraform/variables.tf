variable "region" {
  type        = string
  description = "AWS region to run this in (ex. us-west-2)"
}

variable "domain" {
  type        = string
  description = "Root domain name - should already exist as a zone in AWS. (ex. city-six.com)"
}

variable "subdomain" {
  type        = string
  description = "Subdomain for this environment (ex. uplink)"
}

variable "semver" {
  type        = string
  description = "The version of this service (ex. 0.1.0)"
}

variable "public_key" {
  type        = string
  description = "Instance public key for admin SSH access (contents of)"
}

provider "aws" {
  region = var.region
}