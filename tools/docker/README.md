# zcore Docker Container Setup Guide

## Install Docker

**For Ubuntu Linux:**
https://docs.docker.com/compose/install/

**For Mac OS:**
https://docs.docker.com/docker-for-mac/install/

## Method1: Create docker image and start a new container

```
### Change username, password and timezone
$ vi .env

### Creating new container
$ docker-compose up -d
```

## Method2: Import prebuilt docker image and start a new container

```
### Load prebuilt docker image to system
$ docker load -i xxx-<version>.tar.gz

### Creating new container
$ docker-compose up -d
```

## Login container via SSH

```
$ ./zcore-shell.sh
```

