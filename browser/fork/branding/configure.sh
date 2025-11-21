#!/bin/bash
# AnonNet Browser Branding Configuration
# Defines branding parameters for the build system

MOZ_APP_NAME=anonnet-browser
MOZ_APP_DISPLAYNAME="AnonNet Browser"
MOZ_APP_VENDOR="AnonNet Project"
MOZ_APP_BASENAME=AnonNet
MOZ_APP_VERSION=1.0.0
MOZ_APP_ID="{anonnet-browser-001}"

# Update channel
MOZ_UPDATE_CHANNEL=release

# Distribution ID
MOZ_DISTRIBUTION_ID=org.anonnet

# Branding assets
MOZ_BRANDING_DIRECTORY=browser/branding/anonnet

# App specific settings
export MOZ_APP_NAME
export MOZ_APP_DISPLAYNAME
export MOZ_APP_VENDOR
export MOZ_APP_BASENAME
export MOZ_APP_VERSION
export MOZ_APP_ID
export MOZ_UPDATE_CHANNEL
export MOZ_DISTRIBUTION_ID
export MOZ_BRANDING_DIRECTORY
