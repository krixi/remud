resource "aws_vpc" "remud" {
  cidr_block = "172.16.0.0/16"
}

resource "aws_subnet" "remud" {
  vpc_id            = aws_vpc.remud.id
  cidr_block        = "172.16.10.0/24"
  availability_zone = "${var.region}b"
}

resource "aws_internet_gateway" "remud" {
  vpc_id = aws_vpc.remud.id
}

resource "aws_route_table" "remud" {
  vpc_id = aws_vpc.remud.id

  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.remud.id
  }
}

resource "aws_main_route_table_association" "remud" {
  vpc_id         = aws_vpc.remud.id
  route_table_id = aws_route_table.remud.id
}

resource "aws_security_group" "remud" {
  vpc_id = aws_vpc.remud.id

  ingress {
    description      = "HTTPS from Internet"
    from_port        = 443
    to_port          = 443
    protocol         = "tcp"
    cidr_blocks      = ["0.0.0.0/0"]
    ipv6_cidr_blocks = ["::/0"]
  }

  ingress {
    description      = "HTTP from Internet"
    from_port        = 80
    to_port          = 80
    protocol         = "tcp"
    cidr_blocks      = ["0.0.0.0/0"]
    ipv6_cidr_blocks = ["::/0"]
  }

  ingress {
    description      = "Telnet from Internet"
    from_port        = 23
    to_port          = 23
    protocol         = "tcp"
    cidr_blocks      = ["0.0.0.0/0"]
    ipv6_cidr_blocks = ["::/0"]
  }

  ingress {
    description      = "SSH"
    from_port        = 22
    to_port          = 22
    protocol         = "tcp"
    cidr_blocks      = ["0.0.0.0/0"]
    ipv6_cidr_blocks = ["::/0"]
  }

  egress {
    description      = "All Egress"
    from_port        = 0
    to_port          = 0
    protocol         = "-1"
    cidr_blocks      = ["0.0.0.0/0"]
    ipv6_cidr_blocks = ["::/0"]
  }
}