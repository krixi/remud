data "aws_route53_zone" "city_six" {
  name = var.domain
}

resource "aws_route53_zone" "uplink" {
  name = "${var.subdomain}.${data.aws_route53_zone.city_six.name}"
}

resource "aws_route53_record" "uplink_ns" {
  type    = "NS"
  zone_id = data.aws_route53_zone.city_six.zone_id
  name    = aws_route53_zone.uplink.name
  ttl     = 300
  records = aws_route53_zone.uplink.name_servers
}

resource "aws_route53_record" "uplink" {
  type    = "A"
  zone_id = aws_route53_zone.uplink.zone_id
  name    = "."
  ttl     = 300
  records = [aws_eip.remud.public_ip]
}