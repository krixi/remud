data "aws_ami" "amzn_linux" {
  most_recent = true

  filter {
    name   = "name"
    values = ["amzn2-ami-hvm*"]
  }

  filter {
    name   = "architecture"
    values = ["x86_64"]
  }

  owners = ["137112412989"]
}

resource "aws_network_interface" "remud" {
  subnet_id       = aws_subnet.remud.id
  security_groups = [aws_security_group.remud.id]
}

resource "aws_instance" "remud" {
  ami               = data.aws_ami.amzn_linux.id
  instance_type     = "t2.small"
  availability_zone = "${var.region}b"
  key_name          = aws_key_pair.remud.key_name

  network_interface {
    network_interface_id = aws_network_interface.remud.id
    device_index         = 0
  }

  depends_on = [aws_internet_gateway.remud]

  user_data = <<-EOT
    #! /bin/sh
    yum update -y
    amazon-linux-extras install docker -y
    service docker start
    usermod -a -G docker ec2-user
    chkconfig docker on
  EOT
}

resource "aws_eip" "remud" {
  instance = aws_instance.remud.id
  vpc      = true

  depends_on = [aws_internet_gateway.remud]
}

resource "aws_ebs_volume" "remud" {
  availability_zone = aws_instance.remud.availability_zone
  size              = 5

  lifecycle {
    prevent_destroy = true
  }
}

resource "aws_volume_attachment" "remud" {
  device_name = "/dev/sdg"
  volume_id   = aws_ebs_volume.remud.id
  instance_id = aws_instance.remud.id
}

resource "aws_key_pair" "remud" {
  key_name   = "admin-key"
  public_key = var.public_key
}

# resource "aws_iam_role" "remud-server" {
#  name = "remud_server"

#  assume_role_policy = jsonencode({
#    Version = "2012-10-17"
#
#    Statement = [
#      {
#        Action = "sts::AssumeRole"
#        Effect = "Allow"
#        Sid = ""
#        Principal = {
#          Service = "ec2.amazonaws.com"
#        }
#      }
#    ]
#  })
#}